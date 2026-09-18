# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import unittest
from verification import request, request_digest, result, VerificationError


class VerificationContractTests(unittest.TestCase):
    def setUp(self):
        self.request = {"schema": "oh.war/verification-request/v1",
            "verification_id": "00000000-0000-4000-8000-000000000001",
            "warrant_id": "00000000-0000-4000-8000-000000000002", "source": "exact source",
            "source_sha256": hashlib.sha256(b"exact source").hexdigest(),
            "candidate_revision": "b" * 40, "policy_sha256": "c" * 64,
            "performer": "configured-worker", "verifier": "configured-reviewer", "checks": [["check-api"]]}
        self.result = {"schema": "oh.war/verification-result/v1", "verification_id": self.request["verification_id"],
            "request_sha256": request_digest(self.request), "verdict": "pass", "summary": "Observed expected behavior", "findings": []}
        self.finding = {"id": "F-1", "observation": "Missing required response", "scope": "API fixture",
                        "evidence": ["fixture received 500 instead of 200"], "status": "violation", "repairable": True}

    def test_exact_result_is_copied_without_qualification(self):
        parsed = result(self.result, self.request)
        self.result["summary"] = "Caller changed"
        self.assertEqual(parsed["summary"], "Observed expected behavior")
        self.assertNotIn("qualified", parsed)

    def test_every_changed_candidate_basis_invalidates_previous_result(self):
        for key, replacement in (("candidate_revision", "d" * 40), ("policy_sha256", "e" * 64),
                                 ("checks", [["different-check"]]), ("verifier", "another-verifier"),
                                 ("performer", "another-worker")):
            with self.subTest(key=key), self.assertRaises(VerificationError):
                result(self.result, {**self.request, key: replacement})
        with self.assertRaises(VerificationError):
            result({**self.result, "verification_id": self.request["warrant_id"]}, self.request)

    def test_self_verifier_changed_source_and_unbounded_input_refuse(self):
        for patch in ({"verifier": self.request["performer"]}, {"source": "changed"}, {"checks": []},
                      {"source": "x" * 65537}, {"checks": [["bad\0command"]]}, {"qualified": True}):
            with self.subTest(patch=list(patch)), self.assertRaises(VerificationError):
                request({**self.request, **patch})

    def test_unknown_and_observed_failure_remain_distinct(self):
        unknown = {**self.result, "verdict": "unknown", "summary": "Check unavailable"}
        self.assertEqual(result(unknown, self.request)["verdict"], "unknown")
        failed = {**self.result, "verdict": "fail", "findings": [self.finding]}
        self.assertEqual(result(failed, self.request)["findings"][0]["status"], "violation")
        for patch in ({"verdict": "pass", "findings": [self.finding]}, {"verdict": "fail"},
                      {"qualified": True}, {"actor": "human"}, {"findings": [self.finding] * 2}):
            with self.subTest(patch=patch), self.assertRaises(VerificationError):
                result({**self.result, **patch}, self.request)
        bad = copy.deepcopy(failed);bad["findings"][0]["status"] = "unknown"
        with self.assertRaises(VerificationError): result(bad, self.request)
        bad["verdict"] = "unknown";bad["findings"][0]["repairable"] = False
        self.assertEqual(result(bad, self.request)["verdict"], "unknown")


if __name__ == "__main__":
    unittest.main()
