# SPDX-License-Identifier: Apache-2.0
import copy
import unittest
from verification import VerificationError
from verifier_policy import policy, admission, PROTECTIONS


class VerifierPolicyTests(unittest.TestCase):
    def setUp(self):
        self.config = {"schema": "oh.war/verifier-config/v1", "performer": {"id": "worker", "argv": ["worker"]},
            "verifier": {"id": "reviewer", "argv": ["reviewer"]}, "cost_mode": "free", "spend_limit_usd": 10,
            "timeout_seconds": 60}
        self.policy = {"source_sha256": "a" * 64, "checks": [["check"]]}
        self.attempt = {"attempt_id": "fixture", "source_sha256": "a" * 64, "sequence": 2,
            "work_state": "completed", "execution_state": "stopped", "result_revision": "b" * 40,
            "policy": self.policy, "harness_argv": ["worker"], "checks": [{"argv": ["check"], "exit_code": 0}]}

    def admit(self, **kwargs): return admission(self.config, self.attempt, self.policy, "a" * 64, **kwargs)

    def test_missing_evidence_is_unknown_and_ready_never_grants_dispatch(self):
        missing = self.admit();self.assertEqual(missing["state"], "unknown")
        evidence = {"basis_sha256": missing["basis_sha256"], "evidence_ref": "fixture://trusted-observation",
                    "protections": {p: "pass" for p in PROTECTIONS}}
        ready = self.admit(trusted_observation=evidence)
        self.assertEqual(ready["state"], "ready")
        self.assertFalse(ready["dispatch_permitted"]);self.assertFalse(ready["qualified"])
        for status, expected in [("fail", "blocked"), ("unknown", "unknown")]:
            modified = copy.deepcopy(evidence);modified["protections"]["protected-checks"] = status
            self.assertEqual(self.admit(trusted_observation=modified)["state"], expected)
        self.attempt["result_revision"] = "c" * 40
        self.assertEqual(self.admit(trusted_observation=evidence)["state"], "unknown")

    def test_stale_policy_wrong_performer_and_unknown_cap_refuse(self):
        self.attempt["harness_argv"] = ["other"]
        self.assertEqual(self.admit()["state"], "blocked")
        self.attempt["harness_argv"] = ["worker"];self.config["cost_mode"] = "unknown"
        self.assertEqual(self.admit()["state"], "blocked")
        self.config["spend_limit_usd"] = None
        self.assertEqual(self.admit()["state"], "unknown")
        self.policy = {**self.policy, "checks": [["new-check"]]}
        self.assertEqual(self.admit()["state"], "blocked")

    def test_configuration_cannot_invent_capabilities_or_self_verification(self):
        for patch in ({"protections": list(PROTECTIONS)}, {"timeout_seconds": True},
                      {"spend_limit_usd": float('nan')}, {"spend_limit_usd": float('inf')},
                      {"verifier": {"id": "worker", "argv": ["reviewer"]}}):
            with self.subTest(patch=patch), self.assertRaises(VerificationError): policy({**self.config, **patch})
