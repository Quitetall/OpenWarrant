# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import json
import subprocess
import sys
import threading
import time
import unittest

from hotline import digest
from server import publish, read_file, decode
from test_verifier_workspace import GitFixture
from verification import VerificationError
from verifier_attestation import NAMESPACE
from verifier_controller import run
from verifier_jobs import Jobs
from verifier_policy import admission, PROTECTIONS


class VerifierControllerTests(GitFixture, unittest.TestCase):
    def setUp(self):
        super().setUp()
        self.key = self.root / "fixture-key"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(self.key)], check=True)
        public = " ".join(self.key.with_suffix('.pub').read_text().split()[:2])
        self.program = self.root / "reviewer.py"
        self.program.write_text("""import json,sys,hashlib
r=json.load(sys.stdin)
h=hashlib.sha256(json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
print(json.dumps({'schema':'oh.war/verification-result/v1','verification_id':r['verification_id'],'request_sha256':h,'verdict':'pass','summary':'Fixture observation','findings':[]}))
""")
        source = hashlib.sha256(b"source").hexdigest()
        config = {"schema": "oh.war/verifier-config/v1", "performer": {"id": "worker", "argv": ["worker"]},
                  "verifier": {"id": "reviewer", "argv": [sys.executable, str(self.program)]},
                  "cost_mode": "free", "spend_limit_usd": 10, "timeout_seconds": 5}
        policy = {"source_sha256": source, "checks": [[sys.executable, "-c", "assert open('code.txt').read()=='candidate'"]]}
        self.expected = {"schema": "oh.war/verification-request/v1",
            "verification_id": "00000000-0000-4000-8000-000000000001",
            "warrant_id": "00000000-0000-4000-8000-000000000002", "source": "source",
            "source_sha256": source, "candidate_revision": self.revision, "policy_sha256": digest(policy),
            "performer": "worker", "verifier": "reviewer", "checks": policy["checks"]}
        attempt = {"attempt_id": "fixture", "warrant_id": self.expected["warrant_id"], "source_sha256": source,
            "sequence": 2, "work_state": "completed", "execution_state": "stopped", "result_revision": self.revision,
            "policy": policy, "harness_argv": ["worker"], "checks": [{"argv": policy["checks"][0], "exit_code": 0}]}
        self.current = {"config": config, "attempt": attempt, "execution_policy": policy, "source_sha256": source,
                        "source_path": str(self.source), "issuer": {"public_key": public, "principal": "fixture"}}
        self.jobs = Jobs(self.root / "jobs", publish=publish, read_file=read_file, decode=decode)
        basis = admission(config, attempt, policy, source)["basis_sha256"]
        self.jobs.claim(self.expected, basis)
        now = int(time.time())
        self.payload = json.dumps({"schema": "oh.war/harness-protection/v1", "basis_sha256": basis,
            "nonce": self.expected["verification_id"], "issued_at_unix": now, "expires_at_unix": now + 120,
            "evidence_ref": "fixture://synthetic", "protections": {p: "pass" for p in PROTECTIONS}}).encode()
        self.signature = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", str(self.key), "-n", NAMESPACE],
            input=self.payload, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout

    def dispatch(self, snapshot=None):
        return run(self.jobs, self.expected, snapshot or (lambda: copy.deepcopy(self.current)),
                   threading.RLock(), self.destination, payload=self.payload, signature=self.signature)

    def test_real_process_success_is_durable_and_replay_does_not_launch(self):
        first = self.dispatch()
        self.assertEqual(first["record"]["observation"]["verdict"], "pass")
        self.program.write_text("raise RuntimeError('must not launch again')")
        self.assertEqual(self.dispatch(), first)
        self.assertFalse(first["qualified"])

    def test_changed_policy_after_verifier_pass_becomes_unknown(self):
        calls = 0
        def snapshot():
            nonlocal calls
            calls += 1
            state = copy.deepcopy(self.current)
            if calls > 1: state["config"]["timeout_seconds"] = 6
            return state
        observed = self.dispatch(snapshot)["record"]["observation"]
        self.assertEqual(observed["verdict"], "unknown")
        self.assertEqual(observed["candidate_observation"]["verdict"], "pass")

    def test_dirty_candidate_refuses_before_claim_consumption(self):
        (self.source / "code.txt").write_text("changed")
        with self.assertRaises(VerificationError): self.dispatch()
        self.assertEqual(self.jobs.read(self.expected["verification_id"])["sequence"], 1)
        self.assertFalse(self.destination.exists())

    def test_mutated_performer_workspace_invalidates_otherwise_passing_result(self):
        self.program.write_text(f"from pathlib import Path;Path({str(self.source / 'code.txt')!r}).write_text('changed')\n"
                                + self.program.read_text())
        observed = self.dispatch()["record"]["observation"]
        self.assertEqual(observed["verdict"], "unknown")
        self.assertEqual(observed["candidate_observation"]["verdict"], "pass")
