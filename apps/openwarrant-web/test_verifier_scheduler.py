# SPDX-License-Identifier: Apache-2.0
import json
import time
import unittest
import threading
import urllib.request
import urllib.error

import test_verifier_loop as loop_tests
from test_verifier_repair_execution import RepairExecutionFixture
from verifier_scheduler import Scheduler
from verification import VerificationError
from server import Server


class SchedulerTests(RepairExecutionFixture, unittest.TestCase):
    receipt = loop_tests.VerifierLoopTests.receipt

    def scheduler(self, receipt=None):
        result = Scheduler(self.service, receipt or (lambda job: None), interval=.02)
        self.addCleanup(result.close)
        return result

    def test_http_loop_registration_pause_and_authentication(self):
        scheduler = self.scheduler()
        server = Server(('127.0.0.1', 0), self.store, 'fixture-token')
        server.verification_loops = scheduler
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        def call(fields=None, token='fixture-token'):
            req = urllib.request.Request(f'http://127.0.0.1:{server.server_port}/api/verification-loops',
                data=json.dumps(fields).encode() if fields else None,
                headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json'})
            try: response = urllib.request.urlopen(req, timeout=5)
            except urllib.error.HTTPError as error: response = error
            with response: return response.status, json.loads(response.read())
        try:
            fields = {'attempt_id': self.original['attempt_id'], 'enabled': True}
            self.assertEqual(call(fields, token='wrong')[0], 401)
            self.assertEqual(call()[1]['loops'], [])
            self.assertEqual(call({**fields, 'argv': ['sh']})[0], 409)
            code, enabled = call(fields)
            self.assertEqual(code, 200); self.assertTrue(enabled['enabled'])
            scheduler.poll()
            self.assertEqual(call()[1]['loops'][0]['observation']['state'], 'waiting_for_protection')
            self.assertEqual(call({**fields, 'enabled': False})[1]['state'], 'paused')
            self.assertFalse(call()[1]['loops'][0]['enabled'])
        finally:
            server.shutdown(); server.server_close(); thread.join(timeout=3)

    def test_background_loop_completes_and_pause_survives_restart(self):
        scheduler = self.scheduler(self.receipt)
        fields = {'attempt_id': self.original['attempt_id'], 'enabled': True}
        first = scheduler.configure(fields)
        self.assertEqual(scheduler.configure(fields), first)
        scheduler.start()
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            rows = scheduler.listing()['loops']
            if rows[0]['observation'] and rows[0]['observation']['state'] == 'checks_passed': break
            time.sleep(.02)
        self.assertEqual(rows[0]['observation']['state'], 'checks_passed', rows)
        paused = scheduler.configure({**fields, 'enabled': False})
        self.assertEqual(paused['state'], 'paused')
        scheduler.close()
        restarted = self.scheduler(lambda job: self.fail('Paused loop asked for a receipt'))
        restarted.poll()
        row = restarted.listing()['loops'][0]
        self.assertFalse(row['enabled']); self.assertIsNone(row['observation'])
        self.assertEqual(len(self.executor.records()), 2)
        self.assertEqual(len(self.service.listing()['jobs']), 2)

    def test_registration_alone_does_not_dispatch_and_corrupt_history_refuses(self):
        scheduler = self.scheduler()
        fields = {'attempt_id': self.original['attempt_id'], 'enabled': True}
        scheduler.configure(fields)
        self.assertEqual(self.service.listing()['jobs'], [])
        scheduler.poll()
        self.assertEqual(scheduler.listing()['loops'][0]['observation']['state'], 'waiting_for_protection')
        path = next(scheduler.root.glob('*.json'))
        envelope = json.loads(path.read_text()); envelope['record']['enabled'] = False
        path.write_text(json.dumps(envelope))
        with self.assertRaises(VerificationError): scheduler.poll()
        self.assertEqual(len(self.executor.records()), 1)

    def test_injected_configuration_refuses(self):
        scheduler = self.scheduler()
        for fields in ({'attempt_id': self.original['attempt_id'], 'enabled': True, 'receipt_argv': ['sh']},
                       {'attempt_id': self.original['attempt_id'], 'enabled': 'true'}):
            with self.assertRaises(VerificationError): scheduler.configure(fields)
        self.assertEqual(scheduler.listing()['loops'], [])
