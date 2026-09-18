# SPDX-License-Identifier: Apache-2.0
import base64
import json
import subprocess
import time
import unittest
from pathlib import Path

from test_verifier_repair_execution import RepairExecutionFixture
from verifier_attestation import NAMESPACE
from verifier_policy import PROTECTIONS
from verifier_loop import Loop
from verification import VerificationError


class VerifierLoopTests(RepairExecutionFixture, unittest.TestCase):
    def receipt(self, job):
        now = int(time.time())
        payload = json.dumps({'schema': 'oh.war/harness-protection/v1',
            'basis_sha256': job['basis_sha256'], 'nonce': job['verification_id'],
            'issued_at_unix': now, 'expires_at_unix': now + 120,
            'evidence_ref': 'fixture://loop', 'protections': {p: 'pass' for p in PROTECTIONS}}).encode()
        signature = subprocess.run(['ssh-keygen', '-Y', 'sign', '-f', str(self.key), '-n', NAMESPACE],
            input=payload, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout
        return {'payload_base64': base64.b64encode(payload).decode(),
                'signature_base64': base64.b64encode(signature).decode()}

    def drive(self, loop):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            status = loop.tick(self.original['attempt_id'])
            if status['state'] in ('checks_passed', 'needs_attention'):
                return status
            time.sleep(.02)
        self.fail('Loop did not settle')

    def test_automatic_repair_and_reverification_survive_driver_restart(self):
        loop = Loop(self.service, lambda job: None)
        first = loop.tick(self.original['attempt_id'])
        self.assertEqual(first['state'], 'waiting_for_protection')
        self.assertEqual(loop.tick(self.original['attempt_id']), first)
        self.assertEqual(len(self.service.listing()['jobs']), 1)
        result = self.drive(Loop(self.service, self.receipt))
        self.assertEqual(result['state'], 'checks_passed', result)
        self.assertFalse(result['qualified'])
        self.assertEqual(len(self.executor.records()), 2)
        jobs = self.service.listing()['jobs']
        self.assertEqual(sorted(j['effective_verdict'] for j in jobs), ['fail', 'pass'])
        restarted = Loop(self.service, lambda job: self.fail('Completed loop requested another receipt'))
        self.assertEqual(restarted.tick(self.original['attempt_id']), result)
        self.assertEqual(len(self.service.listing()['jobs']), 2)
        current = self.executor.get(result['attempt_id'])
        (Path(current['worktree']) / 'unreviewed.txt').write_text('changed after verification')
        with self.assertRaises(ValueError):
            restarted.tick(self.original['attempt_id'])

    def test_zero_repairs_stop_without_writer(self):
        self.executor.config['repair_cycles'] = 0
        result = self.drive(Loop(self.service, self.receipt))
        self.assertEqual(result['state'], 'needs_attention')
        self.assertIn('limit', result['reason'])
        self.assertEqual(len(self.executor.records()), 1)

    def test_consumed_claim_without_live_worker_does_not_relaunch(self):
        # Simulate loss after durable consume but before worker launch.
        self.service.schedule = lambda id, work: self.service.get(id)
        first = Loop(self.service, self.receipt).tick(self.original['attempt_id'])
        self.assertEqual(first['state'], 'waiting_for_verification')
        restarted = Loop(self.service, lambda job: self.fail('Consumed claim requested another receipt'))
        result = restarted.tick(self.original['attempt_id'])
        self.assertEqual(result['state'], 'waiting_for_verification')
        self.assertEqual(result['reason'], 'unknown')
        self.assertEqual(len(self.service.listing()['jobs']), 1)
        self.assertEqual(len(self.executor.records()), 1)

    def test_missing_or_forged_receipt_never_consumes_claim(self):
        loop = Loop(self.service, lambda job: {'payload_base64': 'e30=', 'signature_base64': 'e30='})
        with self.assertRaises(VerificationError):
            loop.tick(self.original['attempt_id'])
        job = self.service.listing()['jobs'][0]
        self.assertEqual(job['state'], 'prepared')
        self.assertEqual(len(self.executor.records()), 1)
