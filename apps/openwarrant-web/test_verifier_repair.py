# SPDX-License-Identifier: Apache-2.0
import unittest

from verification import VerificationError
from verifier_repair import plan


class VerifierRepairTests(unittest.TestCase):
    def setUp(self):
        self.finding = {'id': 'F1', 'status': 'violation', 'repairable': True, 'observation': 'Wrong response',
                        'scope': 'Fixture API', 'evidence': ['response was 500']}
        self.job = {'verification_id': 'v1', 'request': {'warrant_id': 'war', 'candidate_revision': 'a' * 40},
                    'evidence_state': 'retained', 'effective_verdict': 'fail', 'record': {'sequence': 3,
                    'observation': {'execution_state': 'stopped', 'result': {'findings': [self.finding]}, 'checks': []}}}
        self.repairs = [{'warrant_id': 'war', 'attempt_id': 'r1',
                         'verification_repair': {'verification_id': 'v0'}}]

    def test_repairable_failure_keeps_default_three_and_no_dispatch_grant(self):
        result = plan(self.job, self.repairs)
        self.assertEqual((result['state'], result['repair_cycles'], result['cycles_used']), ('ready', 3, 1))
        self.assertFalse(result['qualified']);self.assertFalse(result['dispatch_permitted'])

    def test_selected_zero_or_one_limit_overrides_default(self):
        self.assertEqual(plan(self.job, [], 0)['state'], 'exhausted')
        self.assertEqual(plan(self.job, self.repairs, 1)['state'], 'exhausted')
        for limit in (True, -1, 21, None):
            with self.subTest(limit=limit), self.assertRaises(VerificationError): plan(self.job, [], limit)

    def test_resume_and_other_warrants_do_not_consume_repair_cycles(self):
        attempts = self.repairs + [{**self.repairs[0], 'attempt_id': 'resume', 'resume_from': 'r1'},
                                   {**self.repairs[0], 'warrant_id': 'other'}]
        self.assertEqual(plan(self.job, attempts)['cycles_used'], 1)
        self.assertEqual(plan(self.job, attempts)['state'], 'ready')

    def test_same_verification_cannot_dispatch_second_repair(self):
        self.repairs[0]['verification_repair']['verification_id'] = 'v1'
        result = plan(self.job, self.repairs, 0)
        self.assertEqual(result['state'], 'already_dispatched')
        self.assertEqual(result['attempt_id'], 'r1')
        with self.assertRaises(VerificationError): plan(self.job, self.repairs * 2)

    def test_unknown_nonrepairable_missing_evidence_and_live_verifier_block(self):
        self.finding['repairable'] = False
        self.assertEqual(plan(self.job, [])['state'], 'escalate')
        self.finding.update(status='unknown', repairable=True)
        self.assertEqual(plan(self.job, [])['state'], 'escalate')
        self.job['effective_verdict'] = 'unknown'
        self.assertEqual(plan(self.job, [])['state'], 'escalate')
        self.job['evidence_state'] = 'unavailable'
        self.assertEqual(plan(self.job, [])['state'], 'blocked')
        self.job['evidence_state'] = 'retained';self.job['record']['observation']['execution_state'] = 'unknown'
        self.assertEqual(plan(self.job, [])['state'], 'blocked')

    def test_failed_protected_check_can_request_repair_but_pass_cannot(self):
        observed = self.job['record']['observation'];observed['result'] = None
        observed['checks'] = [{'argv': ['test'], 'exit_code': 7}]
        self.assertEqual(plan(self.job, [])['state'], 'ready')
        observed['checks'][0]['exit_code'] = False
        with self.assertRaises(VerificationError): plan(self.job, [])
        self.job['effective_verdict'] = 'pass'
        self.assertEqual(plan(self.job, [])['state'], 'not_required')

    def test_human_repair_decision_preserves_limits_evidence_and_unknown_refusal(self):
        self.job['request']['schema'] = 'oh.war/verification-request/v2'
        self.finding['repairable'] = False
        self.assertEqual(plan(self.job, [])['state'], 'escalate')
        self.assertEqual(plan(self.job, [], human_repair=True)['state'], 'ready')
        self.assertEqual(plan(self.job, [], 0, human_repair=True)['state'], 'exhausted')
        self.finding['status'] = 'unknown'
        self.assertEqual(plan(self.job, [], human_repair=True)['state'], 'escalate')
        self.job['evidence_state'] = 'unavailable'
        self.assertEqual(plan(self.job, [], human_repair=True)['state'], 'blocked')
