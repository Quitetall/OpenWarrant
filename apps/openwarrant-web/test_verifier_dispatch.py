# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from hotline import digest
from server import publish, read_file, decode
from verification import VerificationError
from verifier_attestation import NAMESPACE
from verifier_dispatch import consume
from verifier_jobs import Jobs
from verifier_policy import admission, PROTECTIONS


class VerifierDispatchTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        root = Path(self.tmp.name);self.key = root / "fixture-machine-key"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(self.key)], check=True)
        self.public = " ".join(self.key.with_suffix('.pub').read_text().split()[:2])
        self.jobs = Jobs(root / "jobs", publish=publish, read_file=read_file, decode=decode)
        self.source = hashlib.sha256(b"source").hexdigest()
        self.policy = {"source_sha256": self.source, "checks": [["check"]]}
        self.config = {"schema": "oh.war/verifier-config/v1", "performer": {"id": "worker", "argv": ["worker"]},
            "verifier": {"id": "reviewer", "argv": ["reviewer"]}, "cost_mode": "free",
            "spend_limit_usd": 10, "timeout_seconds": 60}
        self.expected = {"schema": "oh.war/verification-request/v1",
            "verification_id": "00000000-0000-4000-8000-000000000001",
            "warrant_id": "00000000-0000-4000-8000-000000000002", "source": "source",
            "source_sha256": self.source, "candidate_revision": "b" * 40, "policy_sha256": digest(self.policy),
            "performer": "worker", "verifier": "reviewer", "checks": [["check"]]}
        self.attempt = {"attempt_id": "fixture", "warrant_id": self.expected["warrant_id"],
            "source_sha256": self.source, "sequence": 2, "work_state": "completed", "execution_state": "stopped",
            "result_revision": "b" * 40, "policy": self.policy, "harness_argv": ["worker"],
            "checks": [{"argv": ["check"], "exit_code": 0}]}
        basis = admission(self.config, self.attempt, self.policy, self.source)["basis_sha256"]
        self.jobs.claim(self.expected, basis)
        self.receipt = {"schema": "oh.war/harness-protection/v1", "basis_sha256": basis,
            "nonce": self.expected["verification_id"], "issued_at_unix": 1000, "expires_at_unix": 1100,
            "evidence_ref": "fixture://synthetic-protections", "protections": {p: "pass" for p in PROTECTIONS}}

    def signed(self):
        payload = json.dumps(self.receipt).encode()
        p = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", str(self.key), "-n", NAMESPACE],
                           input=payload, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
        return payload, p.stdout

    def consume(self, signed=None):
        payload, signature = signed or self.signed()
        return consume(self.jobs, self.expected, self.config, self.attempt, self.policy, self.source,
                       payload=payload, signature=signature, public_key=self.public,
                       principal="fixture-harness", now=1050)

    def test_signed_exact_claim_consumes_once_and_retains_signed_bytes(self):
        signed = self.signed()
        self.assertTrue(self.consume(signed))
        self.assertFalse(self.consume(signed))
        prior = self.jobs.read(self.expected["verification_id"])
        retained = decode(read_file(self.jobs.root / f"protection-{prior['protection_sha256']}.json"))
        self.assertEqual(digest(retained), prior["protection_sha256"])
        self.assertEqual(retained["public_key"], self.public)
        self.assertEqual(self.jobs.view(self.expected["verification_id"])["state"], "unknown")

    def test_signed_unknown_or_fail_never_consumes_claim(self):
        for status in ("unknown", "fail"):
            self.receipt["protections"]["protected-checks"] = status
            with self.subTest(status=status), self.assertRaises(VerificationError): self.consume()
            self.assertEqual(self.jobs.read(self.expected["verification_id"])["sequence"], 1)

    def test_forged_or_wrong_nonce_receipt_cannot_launch(self):
        payload, signature = self.signed()
        with self.assertRaises(VerificationError): self.consume((payload.replace(b'pass', b'fail'), signature))
        self.receipt["nonce"] = self.expected["warrant_id"]
        with self.assertRaises(VerificationError): self.consume()
        self.assertEqual(self.jobs.read(self.expected["verification_id"])["sequence"], 1)

    def test_changed_request_or_configuration_cannot_reuse_claim(self):
        signed = self.signed()
        original = copy.deepcopy(self.expected)
        for patch in ({"checks": [["other"]]}, {"candidate_revision": "c" * 40},
                      {"verifier": "impostor"}, {"warrant_id": self.expected["verification_id"]}):
            self.expected = {**original, **patch}
            with self.subTest(patch=patch), self.assertRaises(VerificationError): self.consume(signed)
        self.expected = original
        self.config["timeout_seconds"] = 61
        with self.assertRaises(VerificationError): self.consume(signed)
        self.assertEqual(self.jobs.read(self.expected["verification_id"])["sequence"], 1)
