# SPDX-License-Identifier: Apache-2.0
import hashlib
import json
import tempfile
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from server import publish, read_file, decode
from verification import VerificationError, request_digest
from verifier_jobs import Jobs


class VerifierJobsTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "jobs"
        self.jobs = self.open()
        self.expected = {"schema": "oh.war/verification-request/v1",
            "verification_id": "00000000-0000-4000-8000-000000000001",
            "warrant_id": "00000000-0000-4000-8000-000000000002", "source": "source",
            "source_sha256": hashlib.sha256(b"source").hexdigest(), "candidate_revision": "b" * 40,
            "policy_sha256": "c" * 64, "performer": "worker", "verifier": "reviewer", "checks": [["check"]]}
        self.id = self.expected["verification_id"]
        result = {"schema": "oh.war/verification-result/v1", "verification_id": self.id,
            "request_sha256": request_digest(self.expected), "verdict": "pass", "summary": "Checks passed", "findings": []}
        self.observed = {"schema": "oh.war/verifier-observation/v1", "request_sha256": request_digest(self.expected),
            "execution_state": "stopped", "verdict": "pass", "result": result, "qualified": False,
            "checks": [{"argv": ["check"], "exit_code": 0}]}

    def open(self):
        return Jobs(self.root, publish=publish, read_file=read_file, decode=decode)

    def test_restart_retains_claim_and_never_relaunches_consumed_work(self):
        self.assertTrue(self.jobs.claim(self.expected, "d" * 64))
        self.assertFalse(self.open().claim(self.expected, "d" * 64))
        self.assertEqual(self.open().view(self.id)["state"], "prepared")
        self.assertTrue(self.jobs.begin(self.id, "e" * 64))
        self.assertEqual(self.open().view(self.id)["state"], "unknown")
        self.assertFalse(self.open().begin(self.id, "e" * 64))
        self.jobs.finish(self.id, self.observed)
        self.assertEqual(self.open().view(self.id)["state"], "finished")
        self.assertFalse(self.open().begin(self.id, "e" * 64))
        with self.assertRaises(VerificationError): self.jobs.finish(self.id, self.observed)
        self.assertEqual(len(list(self.root.iterdir())), 3)

    def test_racing_controllers_acquire_exactly_one_launch(self):
        self.jobs.claim(self.expected, "d" * 64)
        with ThreadPoolExecutor(max_workers=8) as pool:
            outcomes = list(pool.map(lambda _: self.open().begin(self.id, "e" * 64), range(16)))
        self.assertEqual(outcomes.count(True), 1)
        self.assertEqual(self.open().view(self.id)["state"], "unknown")

    def test_conflicting_id_stale_result_and_unconsumed_finish_refuse(self):
        self.jobs.claim(self.expected, "d" * 64)
        with self.assertRaises(VerificationError):
            self.jobs.claim({**self.expected, "candidate_revision": "f" * 40}, "d" * 64)
        with self.assertRaises(VerificationError): self.jobs.finish(self.id, self.observed)
        self.jobs.begin(self.id, "e" * 64)
        for patch in ({"request_sha256": "f" * 64}, {"qualified": True}, {"result": None},
                      {"execution_state": "unknown"}, {"checks": []},
                      {"checks": [{"argv": ["other"], "exit_code": 0}]},
                      {"checks": [{"argv": ["check"], "exit_code": False}]},
                      {"verdict": "fail", "result": None}):
            with self.subTest(patch=patch), self.assertRaises(VerificationError):
                self.jobs.finish(self.id, {**self.observed, **patch})
        self.assertEqual(self.open().view(self.id)["state"], "unknown")

    def test_deleted_predecessor_and_tampered_record_are_not_success(self):
        self.jobs.claim(self.expected, "d" * 64)
        self.jobs.begin(self.id, "e" * 64)
        path = self.jobs.path(self.id, 1)
        original = path.read_bytes()
        altered = json.loads(original);altered["record"]["basis_sha256"] = "f" * 64
        path.write_text(json.dumps(altered))
        with self.assertRaises(VerificationError): self.open().view(self.id)
        path.unlink()
        with self.assertRaises(VerificationError): self.open().view(self.id)
        path.write_bytes(original)
        self.jobs.finish(self.id, self.observed)
        self.jobs.path(self.id, 2).unlink()
        with self.assertRaises(VerificationError): self.open().view(self.id)

    def test_unknown_outcome_and_symlink_refusal(self):
        self.jobs.claim(self.expected, "d" * 64)
        self.jobs.begin(self.id, "e" * 64)
        unknown = {**self.observed, "execution_state": "unknown", "verdict": "unknown", "result": None}
        self.jobs.finish(self.id, unknown)
        self.assertEqual(self.open().view(self.id)["record"]["observation"]["verdict"], "unknown")
        link = Path(self.tmp.name) / "linked"
        link.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(VerificationError):
            Jobs(link, publish=publish, read_file=read_file, decode=decode)
