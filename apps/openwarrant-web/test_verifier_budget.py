# SPDX-License-Identifier: Apache-2.0
import copy
import unittest

from verification import VerificationError
from verifier_budget import allowance


class VerifierBudgetTests(unittest.TestCase):
    def setUp(self):
        self.execution = {'timeout_seconds': 10, 'spend_limit_usd': 10}
        self.verifier = {'timeout_seconds': 8, 'spend_limit_usd': 5, 'cost_mode': 'free'}
        self.attempts = [{'warrant_id': 'war', 'execution_state': 'stopped', 'active_seconds': 3, 'cost_usd': 1}]
        self.jobs = [{'request': {'warrant_id': 'war'}, 'evidence_state': 'retained', 'record': {'sequence': 3,
                     'observation': {'execution_state': 'stopped', 'active_seconds': 2, 'cost_usd': 0}}}]

    def budget(self): return allowance('war', self.execution, self.verifier, self.attempts, self.jobs)

    def test_aggregate_across_ids_and_repairs_never_resets(self):
        self.assertEqual(self.budget()['remaining_seconds'], 5)
        self.attempts.append({**self.attempts[0], 'active_seconds': 2})
        self.jobs.append(copy.deepcopy(self.jobs[0]))
        self.assertEqual(self.budget()['remaining_seconds'], 1)
        self.assertEqual(self.budget()['known_cost_usd'], 2)
        self.jobs.append(copy.deepcopy(self.jobs[0]))
        with self.assertRaisesRegex(VerificationError, 'time budget exhausted'): self.budget()

    def test_missing_invalid_or_uncertain_usage_never_counts_as_zero(self):
        for value in (None, -1, float('nan'), float('inf'), True):
            self.attempts[0]['active_seconds'] = value
            with self.subTest(value=value), self.assertRaises(VerificationError): self.budget()
        self.attempts[0]['active_seconds'] = 3
        for change in ({'sequence': 2}, {'sequence': 3, 'observation': {'execution_state': 'unknown'}}):
            self.jobs[0]['record'] = change
            with self.subTest(change=change), self.assertRaises(VerificationError): self.budget()

    def test_unknown_cost_refuses_either_hard_cap_and_stays_unknown_when_unlimited(self):
        self.attempts[0]['cost_usd'] = None
        with self.assertRaisesRegex(VerificationError, 'Unknown cost'): self.budget()
        self.verifier['spend_limit_usd'] = None
        with self.assertRaisesRegex(VerificationError, 'Unknown cost'): self.budget()
        self.execution['spend_limit_usd'] = None
        self.assertIsNone(self.budget()['cost_usd'])
        self.assertEqual(self.budget()['known_cost_usd'], 0)

    def test_prepared_and_other_warrants_do_not_spend_this_warrant_budget(self):
        self.jobs.append({'request': {'warrant_id': 'war'}, 'record': {'sequence': 1}})
        self.attempts.append({'warrant_id': 'other', 'execution_state': 'running'})
        self.assertEqual(self.budget()['remaining_seconds'], 5)
        self.attempts[0]['cost_usd'] = 6
        with self.assertRaisesRegex(VerificationError, 'spend budget exhausted'): self.budget()
