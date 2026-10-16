# SPDX-License-Identifier: Apache-2.0
"""Public CLI report controls; no provider, model, or real authority involved."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


WAR = None
BODY = "# Fixture\n\n## Decision\nKeep the source.\n\n## Completion\nComplete.\n"


class MigrationReport(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="war-migration-report-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.corpus = self.root / "docs" / "adr"
        self.corpus.mkdir(parents=True)
        (self.corpus / "0001-fixture.md").write_text(BODY)
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        self.env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null")
        self.git("init", "-q")
        self.git("add", "docs")
        self.git("commit", "-qm", "Synthetic migration fixture")
        self.commit = self.git("rev-parse", "HEAD").stdout.strip()

    def git(self, *args):
        return subprocess.run(
            ["git", "-C", str(self.root), "-c", "user.name=Fixture",
             "-c", "user.email=fixture@invalid", "-c", "commit.gpgSign=false", *args],
            capture_output=True, text=True, env=self.env, timeout=15, check=True,
        )

    def invoke(self, *args):
        return subprocess.run(
            [str(WAR), "migrate", "--corpus", str(self.corpus),
             "--commit", self.commit, "--out", str(self.root / "import.json"), *args],
            cwd=self.root, env=self.env, capture_output=True, text=True, timeout=20,
        )

    def test_json_is_one_report_and_preserves_artifact_payload(self):
        result = self.invoke("--json")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["schema"], "oh.war/report/v1")
        self.assertEqual(report["command"], "migrate")
        self.assertEqual(report["exit_code"], result.returncode)
        self.assertEqual(report["verdict"], "well_formed")
        artifact = json.loads((self.root / "import.json").read_bytes())
        self.assertEqual(report["result"], artifact)
        self.assertEqual(artifact["adr_count"], 1)
        self.assertEqual(artifact["historical_claims"], 1)
        self.assertEqual(artifact["promoted_resolutions"], 0)
        self.assertEqual(artifact["adrs"][0]["preserved_body"], BODY)

    def test_promotion_control_refuses_before_writing_an_artifact(self):
        result = self.invoke("--json", "--attempt-promotion")
        self.assertEqual(result.returncode, 1, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["schema"], "oh.war/report/v1")
        self.assertEqual(report["command"], "error")
        self.assertEqual(report["exit_code"], result.returncode)
        self.assertEqual(report["verdict"], "not_ready")
        self.assertGreater(report["counts"]["error"], 0)
        self.assertIn("cli.error",
                      [d["rule"] for d in report["diagnostics"]])
        self.assertNotIn("result", report)
        self.assertFalse((self.root / "import.json").exists())
        self.assertEqual((self.corpus / "0001-fixture.md").read_text(), BODY)

    def test_text_mode_and_readonly_verification_remain_compatible(self):
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(result.stdout.startswith(
            f"commit                        {self.commit}\nADRs imported                 1\n"))
        self.assertIn("promoted resolutions          0   (OBL-003 expects 0)", result.stdout)
        artifact = self.root / "import.json"
        original, modified = artifact.read_bytes(), artifact.stat().st_mtime_ns
        checked = self.invoke("--json", "--verify")
        self.assertEqual(checked.returncode, 0, checked.stderr)
        self.assertEqual(json.loads(checked.stdout)["result"], json.loads(original))
        self.assertEqual(artifact.read_bytes(), original)
        self.assertEqual(artifact.stat().st_mtime_ns, modified)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--war", required=True, type=Path)
    options, tests = parser.parse_known_args()
    WAR = options.war.resolve(strict=True)
    unittest.main(argv=[__file__, *tests])
