# SPDX-License-Identifier: Apache-2.0
import base64
import json
import subprocess
import time
import unittest
from pathlib import Path
import hashlib
import uuid

from test_verifier_repair_execution import RepairExecutionFixture
from verifier_attestation import NAMESPACE
from verifier_policy import PROTECTIONS
from verifier_loop import Loop
from verifier_receipts import ReceiptInbox
from verification import VerificationError
from hotline import Answers


class VerifierLoopTests(RepairExecutionFixture, unittest.TestCase):
    def test_loop_follows_rebuttal_and_human_recheck_then_authorized_repair(self):
        root = self.original['attempt_id']
        loop = Loop(self.service, self.receipt)
        prepared = Loop(self.service, lambda job: None).tick(root)
        failed = self.verify(root, self.service.get(prepared['verification_id']))
        child = self.service.rebut(failed['verification_id'], {
            'verification_id': str(uuid.uuid4()), 'argument': 'Recheck fixture contract',
            'finding_ids': ['F1'], 'evidence': ['fixture://contract']})
        rechecked = self.verify(root, child)
        waiting = loop.tick(root)
        self.assertEqual(waiting['state'], 'needs_attention')
        self.assertEqual(waiting['verification_id'], rechecked['verification_id'])
        self.assertEqual(len(self.executor.records()), 1)
        token = 'fixture-only-loop-human-credential-0001'
        answers = Answers(self.executor, {'schema': 'oh.war/hotline-config/v1', 'responders': [{
            'id': 'fixture-human', 'kind': 'human', 'governing_warrants': [self.original['warrant_id']],
            'token_sha256': hashlib.sha256(token.encode()).hexdigest()}]})
        self.service.answers = answers
        def decide(job, action):
            q = self.service.dispute(job['verification_id'], answers)
            self.service.settle(job['verification_id'], {'question_sha256': q['question_sha256'],
                'action': action, 'reason': 'Fixture decision', 'evidence': ['fixture://contract']}, token, answers)
        decide(rechecked, 'verify_again')
        pending = loop.tick(root)
        self.assertEqual(pending['state'], 'waiting_for_protection')
        again = self.verify(root, self.service.get(pending['verification_id']))
        self.assertEqual(loop.tick(root)['state'], 'needs_attention')
        decide(again, 'repair')
        result = self.drive(loop)
        self.assertEqual(result['state'], 'checks_passed', result)
        self.assertEqual(len(self.executor.records()), 2)
        self.assertEqual(sorted(j['effective_verdict'] for j in self.service.listing()['jobs']),
                         ['fail', 'fail', 'fail', 'pass'])

    def test_file_inbox_drives_real_repair_loop_without_producer_calls(self):
        directory = self.fixture.root / 'receipt-inbox'; directory.mkdir()
        inbox = ReceiptInbox(directory, self.executor.decode); self.addCleanup(inbox.close)
        loop = Loop(self.service, inbox)
        deadline = time.monotonic() + 8
        issued = set()
        while time.monotonic() < deadline:
            status = loop.tick(self.original['attempt_id'])
            if status['state'] == 'checks_passed':
                break
            if status['state'] == 'waiting_for_protection':
                id = status['verification_id']
                self.assertNotIn(id, issued)
                issued.add(id)
                # External synthetic harness publishes the response. The driver
                # only reads this file; production issuers must enforce isolation.
                self.executor.publish(directory / (id + '.json'),
                    json.dumps(self.receipt(self.service.get(id))).encode())
            else:
                self.assertIn(status['state'], ('waiting_for_execution', 'waiting_for_verification'))
            time.sleep(.02)
        self.assertEqual(status['state'], 'checks_passed', status)
        self.assertEqual(len(issued), 2)
        self.assertEqual(len(list(directory.glob('*.json'))), 2)
        self.assertEqual(len(self.executor.records()), 2)

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
