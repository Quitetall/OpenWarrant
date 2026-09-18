# SPDX-License-Identifier: Apache-2.0
"""Real Git and process evidence, including rejection of mutating checks."""
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

from stage_checks import checkpoint
from stages import completed_from_evidence, StageError


class StageCheckpointTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="ow-stage-check-")
        self.root = Path(self.tmp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Stage fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        (self.root / "result.txt").write_text("expected")
        self.git("add", "."); self.git("commit", "-qm", "fixture")
        self.revision = self.git("rev-parse", "HEAD")
        self.config = {"schema": "oh.war/execution-stage-plan/v1", "stages": {
            "api": {"title": "API", "outcome": "Expected result", "dependencies": [],
                    "checks": [[sys.executable, "-c", "from pathlib import Path; assert Path('result.txt').read_text()=='expected'"]]},
            "ui": {"title": "UI", "outcome": "Follow API", "dependencies": ["api"],
                   "checks": [[sys.executable, "-c", "pass"]]}}}

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, text=True, stderr=subprocess.DEVNULL).strip()

    def tearDown(self):
        self.tmp.cleanup()

    def observe(self, seconds=3):
        return checkpoint(self.config, "a" * 64, self.revision, self.root, ["ui", "api"], time.monotonic() + seconds)

    def done(self, record):
        return completed_from_evidence(self.config, "a" * 64, self.revision, record)

    def test_real_checks_establish_current_stage_evidence(self):
        r = self.observe()
        self.assertEqual(r["execution_state"], "stopped", r)
        self.assertEqual(list(r["stage_checks"]), ["api", "ui"])
        self.assertEqual(self.done(r), {"api", "ui"})
        self.assertEqual(self.git("status", "--porcelain"), "")

    def test_failing_prerequisite_cannot_establish_passing_successor(self):
        self.config["stages"]["api"]["checks"] = [[sys.executable, "-c", "raise SystemExit(9)"]]
        r = self.observe()
        self.assertEqual(r["execution_state"], "stopped")
        self.assertEqual(r["stage_checks"]["api"][0]["exit_code"], 9)
        self.assertEqual(r["stage_checks"]["ui"][0]["exit_code"], 0)
        self.assertEqual(self.done(r), set())

    def test_mutating_check_is_unknown_even_with_zero_exit(self):
        self.config["stages"]["api"]["checks"] = [[sys.executable, "-c", "from pathlib import Path;Path('result.txt').write_text('changed')"]]
        r = self.observe()
        self.assertEqual(r["execution_state"], "unknown")
        self.assertEqual(self.done(r), set())
        self.assertNotIn("ui", r["stage_checks"])

    def test_dirty_or_moved_revision_refuses_before_check_launch(self):
        (self.root / "dirty.txt").write_text("dirty")
        r = self.observe()
        self.assertEqual(r["stage_checks"], {})
        self.assertEqual(r["execution_state"], "unknown")
        self.git("add", "."); self.git("commit", "-qm", "moved")
        self.assertEqual(self.observe()["stage_checks"], {})

    def test_time_bound_and_missing_prerequisite_refuse(self):
        self.config["stages"]["api"]["checks"] = [[sys.executable, "-c", "import time;time.sleep(5)"]]
        r = self.observe(seconds=0.1)
        self.assertEqual(r["execution_state"], "unknown")
        self.assertEqual(self.done(r), set())
        with self.assertRaises(StageError):
            checkpoint(self.config, "a" * 64, self.revision, self.root, ["ui"], time.monotonic() + 1)


if __name__ == "__main__":
    unittest.main()
