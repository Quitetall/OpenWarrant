# SPDX-License-Identifier: Apache-2.0
import subprocess
import tempfile
import time
import unittest
from pathlib import Path
from verification import VerificationError
from verifier_workspace import prepare, unchanged


class GitFixture:
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="ow-verifier-")
        self.root = Path(self.tmp.name);self.source = self.root / "source";self.source.mkdir()
        self.git(self.source, "init", "-q")
        self.git(self.source, "config", "user.name", "Fixture")
        self.git(self.source, "config", "user.email", "fixture@example.invalid")
        (self.source / "code.txt").write_text("candidate")
        self.git(self.source, "add", ".");self.git(self.source, "commit", "-qm", "candidate")
        self.revision = self.git(self.source, "rev-parse", "HEAD")
        self.destination = self.root / "verifier"

    def tearDown(self): self.tmp.cleanup()

    def git(self, cwd, *args):
        return subprocess.check_output(["git", *args], cwd=cwd, text=True, stderr=subprocess.PIPE).strip()

    def prepare(self): return prepare(self.source, self.destination, self.revision, time.monotonic() + 5)


class VerifierWorkspaceTests(GitFixture, unittest.TestCase):
    def test_exact_candidate_has_separate_refs_and_no_performer_scratch(self):
        (self.source / "scratch.txt").write_text("performer context")
        destination = self.prepare()
        self.assertEqual((destination / "code.txt").read_text(), "candidate")
        self.assertFalse((destination / "scratch.txt").exists())
        self.assertFalse((destination / ".git/objects/info/alternates").exists())
        self.git(destination, "update-ref", "refs/heads/verifier-only", self.revision)
        self.assertNotIn("verifier-only", self.git(self.source, "branch", "--list"))
        unchanged(destination, self.revision, time.monotonic() + 2)

    def test_existing_nested_or_symlink_destination_refuses_without_overwrite(self):
        self.destination.mkdir();(self.destination / "keep").write_text("preserved")
        with self.assertRaises(VerificationError): self.prepare()
        self.assertEqual((self.destination / "keep").read_text(), "preserved")
        with self.assertRaises(VerificationError):
            prepare(self.source, self.source / "nested", self.revision, time.monotonic() + 2)
        alias = self.root / "alias";alias.symlink_to(self.destination)
        with self.assertRaises(VerificationError):
            prepare(self.source, alias, self.revision, time.monotonic() + 2)

    def test_missing_candidate_and_mutation_cannot_pass(self):
        with self.assertRaises(subprocess.CalledProcessError):
            prepare(self.source, self.destination, "0" * 40, time.monotonic() + 2)
        self.assertFalse(self.destination.exists())
        destination = self.prepare();(destination / "code.txt").write_text("mutated")
        with self.assertRaises(VerificationError): unchanged(destination, self.revision, time.monotonic() + 2)
        self.assertEqual((self.source / "code.txt").read_text(), "candidate")
        with self.assertRaises(TimeoutError): unchanged(destination, self.revision, time.monotonic() - 1)


if __name__ == "__main__": unittest.main()
