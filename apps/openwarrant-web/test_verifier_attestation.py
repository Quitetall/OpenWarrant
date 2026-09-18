# SPDX-License-Identifier: Apache-2.0
"""Synthetic machine keys only; no user keys or human signatures."""
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from verification import VerificationError
from verifier_attestation import authenticate, NAMESPACE
from verifier_policy import PROTECTIONS


class VerifierAttestationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="ow-machine-key-fixture-")
        self.root = Path(self.tmp.name);self.key = self.root / "key"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(self.key)], check=True)
        self.public = " ".join(self.key.with_suffix('.pub').read_text().split()[:2])
        self.record = {"schema": "oh.war/harness-protection/v1", "basis_sha256": "a" * 64,
            "nonce": "00000000-0000-4000-8000-000000000001", "issued_at_unix": 1000,
            "expires_at_unix": 1100, "evidence_ref": "fixture://machine-observation",
            "protections": {p: "pass" for p in PROTECTIONS}}

    def tearDown(self): self.tmp.cleanup()

    def sign(self, record=None, namespace=NAMESPACE):
        payload = json.dumps(record or self.record).encode()
        completed = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", str(self.key), "-n", namespace],
                                   input=payload, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
        return payload, completed.stdout

    def verify(self, payload, signature, **kwargs):
        args = dict(public_key=self.public, principal="fixture-harness", basis_sha256="a" * 64,
                    nonce=self.record["nonce"], now=1050, decode=json.loads)
        args.update(kwargs)
        return authenticate(payload, signature, **args)

    def test_signed_machine_observations_preserve_unknown_without_qualification(self):
        self.record["protections"]["protected-checks"] = "unknown"
        parsed = self.verify(*self.sign())
        self.assertEqual(parsed["protections"]["protected-checks"], "unknown")
        self.assertNotIn("qualified", parsed)
        self.assertNotIn("actor", parsed)

    def test_modified_bytes_wrong_key_and_other_signature_namespace_refuse(self):
        payload, signature = self.sign()
        with self.assertRaises(VerificationError): self.verify(payload.replace(b'pass', b'fail'), signature)
        other = self.root / "other"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(other)], check=True)
        public = " ".join(other.with_suffix('.pub').read_text().split()[:2])
        with self.assertRaises(VerificationError): self.verify(payload, signature, public_key=public)
        with self.assertRaises(VerificationError): self.verify(*self.sign(namespace="human-acceptance"))

    def test_stale_claim_time_and_injected_authority_refuse_even_with_valid_signature(self):
        signed = self.sign()
        for patch in ({"basis_sha256": "b" * 64}, {"nonce": "00000000-0000-4000-8000-000000000002"},
                      {"now": 1100}, {"now": 999}):
            with self.subTest(patch=patch), self.assertRaises(VerificationError): self.verify(*signed, **patch)
        for patch in ({"expires_at_unix": 1400}, {"qualified": True}, {"issued_at_unix": True}):
            with self.subTest(patch=patch), self.assertRaises(VerificationError):
                self.verify(*self.sign({**self.record, **patch}))
        with self.assertRaises(VerificationError): self.verify(*signed, principal="fixture\nother")
