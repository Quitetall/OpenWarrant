# SPDX-License-Identifier: Apache-2.0
import copy
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from server import read_file, decode
from verification import VerificationError
from verifier_snapshot import Snapshot


class VerifierSnapshotTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        root = Path(self.tmp.name)
        self.config_path, self.issuer_path = root / "config.json", root / "issuer.json"
        self.config = {"schema": "oh.war/verifier-config/v1", "performer": {"id": "worker", "argv": ["worker"]},
                       "verifier": {"id": "reviewer", "argv": ["reviewer"]}, "cost_mode": "free",
                       "spend_limit_usd": 10, "timeout_seconds": 60}
        self.config_path.write_text(json.dumps(self.config))
        self.issuer_path.write_text(json.dumps({"schema": "oh.war/verifier-issuer/v1",
                                               "public_key": "fixture", "principal": "fixture"}))
        self.id = "00000000-0000-4000-8000-000000000001"
        self.warrant = "00000000-0000-4000-8000-000000000002"
        self.policy = {"source_sha256": "a" * 64, "checks": [["check"]], "dependencies": []}
        self.row = {"attempt_id": self.id, "warrant_id": self.warrant, "source_sha256": "a" * 64,
            "sequence": 2, "work_state": "completed", "execution_state": "stopped", "result_revision": "b" * 40,
            "policy": copy.deepcopy(self.policy), "checks": [{"argv": ["check"], "exit_code": 0}],
            "harness_argv": ["worker"], "worktree": str(root / ("worktree-" + self.warrant))}
        self.records = {self.id: self.row}
        self.source = {"source_sha256": "a" * 64}
        self.executor = SimpleNamespace(root=root, read_file=read_file, decode=decode,
            config={"argv": ["worker"], "warrants": {self.warrant: self.policy}},
            records=lambda: self.records, view=lambda row: row,
            store=SimpleNamespace(get=lambda _: self.source))
        self.snapshot = Snapshot(self.executor, self.id, self.config_path, self.issuer_path)

    def test_reads_fresh_configuration_and_returns_detached_state(self):
        first = self.snapshot()
        first["attempt"]["checks"].clear()
        self.assertEqual(len(self.snapshot()["attempt"]["checks"]), 1)
        self.config["timeout_seconds"] = 61
        self.config_path.write_text(json.dumps(self.config))
        self.assertEqual(self.snapshot()["config"]["timeout_seconds"], 61)

    def test_running_unknown_and_missing_dependency_refuse(self):
        for state in ("running", "unknown"):
            self.records["other"] = {**self.row, "execution_state": state}
            with self.subTest(state=state), self.assertRaises(VerificationError): self.snapshot()
        self.records.pop("other")
        self.policy["dependencies"] = ["missing"]
        self.row["policy"] = copy.deepcopy(self.policy)
        with self.assertRaises(VerificationError): self.snapshot()

    def test_changed_source_policy_harness_and_worktree_refuse(self):
        original = copy.deepcopy(self.row)
        for patch in ({"worktree": str(self.executor.root / "unrelated")}, {"harness_argv": ["other"]},
                      {"policy": {**self.policy, "checks": [["different"]]}}):
            self.row.clear();self.row.update({**copy.deepcopy(original), **patch})
            with self.subTest(patch=patch), self.assertRaises(VerificationError): self.snapshot()
        self.row.clear();self.row.update(original)
        self.source["source_sha256"] = "c" * 64
        with self.assertRaises(VerificationError): self.snapshot()

    def test_config_symlink_and_unsupported_issuer_fields_refuse(self):
        alias = self.executor.root / "config-alias";alias.symlink_to(self.config_path)
        with self.assertRaises(OSError): Snapshot(self.executor, self.id, alias, self.issuer_path)()
        self.issuer_path.write_text(json.dumps({"schema": "oh.war/verifier-issuer/v1",
            "public_key": "fixture", "principal": "fixture", "qualified": True}))
        with self.assertRaises(VerificationError): self.snapshot()
