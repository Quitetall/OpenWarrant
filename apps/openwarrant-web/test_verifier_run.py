# SPDX-License-Identifier: Apache-2.0
import hashlib
import json
import sys
import time
import unittest
from test_verifier_workspace import GitFixture
from verifier_run import observe


class VerifierRunTests(GitFixture, unittest.TestCase):
    def setUp(self):
        super().setUp()
        self.request = {"schema": "oh.war/verification-request/v1",
            "verification_id": "00000000-0000-4000-8000-000000000001",
            "warrant_id": "00000000-0000-4000-8000-000000000002", "source": "exact source",
            "source_sha256": hashlib.sha256(b"exact source").hexdigest(), "candidate_revision": self.revision,
            "policy_sha256": "c" * 64, "performer": "worker", "verifier": "reviewer",
            "checks": [[sys.executable, "-c", "from pathlib import Path;assert Path('code.txt').read_text()=='candidate'"]]}
        self.program = self.root / "verifier.py"
        self.program.write_text("""import json,sys,hashlib
r=json.load(sys.stdin)
h=hashlib.sha256(json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
print(json.dumps({'schema':'oh.war/verification-result/v1','verification_id':r['verification_id'],'request_sha256':h,'verdict':'pass','summary':'Fixture observed exact candidate','findings':[]}))
""")

    def run_verifier(self, seconds=3):
        return observe(self.request, self.source, self.destination, [sys.executable, str(self.program)],
                       time.monotonic() + seconds, json.loads)

    def test_real_check_and_verifier_pass_on_separate_exact_candidate(self):
        r = self.run_verifier()
        self.assertEqual(r["execution_state"], "stopped", r)
        self.assertEqual(r["verdict"], "pass")
        self.assertFalse(r["qualified"])
        self.assertEqual(r["checks"][0]["exit_code"], 0)
        self.assertNotEqual(r["workspace"], str(self.source))

    def test_failed_check_skips_verifier_and_retains_observed_failure(self):
        self.request["checks"] = [[sys.executable, "-c", "raise SystemExit(7)"]]
        marker = self.root / "launched"
        self.program.write_text(f"from pathlib import Path;Path({str(marker)!r}).touch()")
        r = self.run_verifier()
        self.assertEqual(r["verdict"], "fail", r)
        self.assertEqual(r["execution_state"], "stopped")
        self.assertEqual(r["checks"][0]["exit_code"], 7)
        self.assertFalse(marker.exists())
        self.assertIsNone(r["result"])

    def test_verifier_mutation_cannot_return_pass(self):
        self.program.write_text("from pathlib import Path;Path('code.txt').write_text('changed')\n" + self.program.read_text())
        r = self.run_verifier()
        self.assertEqual(r["verdict"], "unknown", r)
        self.assertEqual(r["execution_state"], "unknown")
        self.assertIsNone(r["result"])
        self.assertEqual((self.source / "code.txt").read_text(), "candidate")

    def test_stale_verifier_result_and_timeout_stay_unknown(self):
        self.program.write_text(self.program.read_text().replace("'request_sha256':h", "'request_sha256':'0'*64"))
        r = self.run_verifier()
        self.assertEqual(r["verdict"], "unknown")
        self.assertIn("Stale", r["cause"])
        self.destination = self.root / "timeout-verifier"
        self.program.write_text("import time;time.sleep(5)")
        r = self.run_verifier(seconds=.3)
        self.assertEqual(r["execution_state"], "unknown")
        self.assertEqual(r["verdict"], "unknown")
