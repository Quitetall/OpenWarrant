# SPDX-License-Identifier: Apache-2.0
import unittest
import copy
import json
import threading
import urllib.request

from test_verifier_repair_execution import RepairExecutionFixture
from verifier_reporting import report, attach
from verification import VerificationError
from server import Server


class VerificationReportTests(RepairExecutionFixture, unittest.TestCase):
    def test_offline_report_retains_failed_and_repaired_candidates(self):
        failed = self.verify(self.original['attempt_id'])
        repaired = self.wait_repair(self.service.repair(failed['verification_id'], {}))
        passed = self.verify(repaired['attempt_id'])
        result = report(self.service, repaired['attempt_id'], 'DONE', 'full')
        self.assertEqual(result['schema'], 'oh.war/reference-work-report/v2')
        self.assertFalse(result['qualified'])
        self.assertEqual(result['completion_signal'], 'DONE')
        self.assertEqual(sorted(j['effective_verdict'] for j in result['verification']['jobs']), ['fail', 'pass'])
        self.assertIn(failed['request']['candidate_revision'], result['html'])
        self.assertIn(passed['request']['candidate_revision'], result['html'])
        self.assertIn(repaired['attempt_id'], result['html'])
        self.assertEqual(result, report(self.service, repaired['attempt_id'], 'DONE', 'full'))
        injected = copy.deepcopy(result['verification'])
        bad = next(j for j in injected['jobs'] if j['effective_verdict'] == 'fail')
        bad['record']['observation']['result']['findings'][0]['observation'] = '<script>unsafe()</script>'
        rendered = attach(self.executor.report(repaired['attempt_id'], 'DONE', 'full'), injected)
        self.assertNotIn('<script>unsafe()', rendered['html'])
        self.assertIn('&lt;script&gt;unsafe()', rendered['html'])
        with self.assertRaises(VerificationError):
            attach(result, {**injected, 'qualified': True})
        compact = report(self.service, repaired['attempt_id'], 'DONE', 'minimal')
        self.assertLess(len(compact['text']), len(result['text']))
        self.assertEqual(compact['verification_snapshot_sha256'], result['verification_snapshot_sha256'])
        self.assertEqual(compact['snapshot_sha256'], result['snapshot_sha256'])
        # Evidence loss affects the verification supplement, not execution history.
        consumed = self.service.jobs.decode(self.service.jobs.read_file(self.service.jobs.path(passed['verification_id'], 2)))['record']
        receipt = self.service.jobs.root / ('protection-' + consumed['protection_sha256'] + '.json')
        receipt.rename(receipt.with_suffix('.retained-fixture'))
        lost = report(self.service, repaired['attempt_id'], 'DONE', 'full')
        job = next(j for j in lost['verification']['jobs'] if j['verification_id'] == passed['verification_id'])
        self.assertEqual(job['effective_verdict'], 'unknown')
        self.assertEqual(job['record']['observation']['verdict'], 'pass')
        self.assertNotEqual(lost['verification_snapshot_sha256'], result['verification_snapshot_sha256'])
        self.assertEqual(lost['snapshot_sha256'], result['snapshot_sha256'])

    def test_unverified_report_without_jobs_remains_valid(self):
        result = report(self.service, self.original['attempt_id'], 'DONE', 'full')
        self.assertEqual(result['verification']['jobs'], [])
        self.assertIn('No retained verification jobs', result['html'])
        self.assertFalse(result['qualified'])

    def test_configured_http_report_exports_supplement(self):
        server = Server(('127.0.0.1', 0), self.store, 'fixture-token')
        server.executor = self.executor; server.verification = self.service
        server.completion_word = 'DONE'; server.report_detail = 'full'
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        try:
            req = urllib.request.Request(f'http://127.0.0.1:{server.server_port}/api/runs/'
                + self.original['attempt_id'] + '/report', headers={'Authorization': 'Bearer fixture-token'})
            with urllib.request.urlopen(req, timeout=5) as response:
                body = json.load(response)
            self.assertEqual(body['schema'], 'oh.war/reference-work-report/v2')
            self.assertIn('Verification and repair history', body['html'])
            self.assertFalse(body['qualified'])
        finally:
            server.shutdown(); server.server_close(); thread.join(timeout=3)
