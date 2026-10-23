# SPDX-License-Identifier: Apache-2.0
"""Test observation retention through the integration-driver command line."""
import os
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
STAGES = ("source", "conditions", "selection", "projections", "package", "budgets", "adapter")


class ObservationOutput(unittest.TestCase):
    def test_adapter_refuses_resolved_sdk_override_despite_matching_manifest_pin(self):
        # Cargo is an external protocol facade here. This proves early refusal
        # and no publication, not real provider compatibility or qualification.
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            provider = root / "provider"
            crate = provider / "lamu-openwarrant"
            crate.mkdir(parents=True)
            pin = "4b626f23b3076ff7e764f08eeb15e8bdfa6af5cd"
            (crate / "Cargo.toml").write_text(
                '[package]\nname="lamu-openwarrant"\nversion="0.1.0"\n'
                '[dependencies.openwarrant-core]\n'
                f'git="https://github.com/Quitetall/OpenWarrant.git"\nrev="{pin}"\n')
            tools = root / "tools"
            tools.mkdir()
            metadata = {
                "packages": [
                    {"id": "provider", "name": "lamu-openwarrant", "manifest_path": str(crate / "Cargo.toml")},
                    {"id": "override", "name": "openwarrant-core", "source": None,
                     "manifest_path": str(root / "replacement/Cargo.toml")},
                ],
                "resolve": {"nodes": [{"id": "provider", "deps": [
                    {"name": "openwarrant_core", "pkg": "override"}]}]},
            }
            cargo = tools / "cargo"
            cargo.write_text("#!/usr/bin/env python3\nimport json, sys\n"
                             "if 'metadata' not in sys.argv: raise SystemExit('unexpected provider execution')\n"
                             f"print({json.dumps(json.dumps(metadata))})\n")
            cargo.chmod(0o755)
            output = root / "observation.json"
            result = subprocess.run(
                [sys.executable, str(ROOT / "conformance/integration/adapter/run.py"),
                 "--provider-root", str(provider), "--output", str(output)],
                cwd=root, env={**os.environ, "PATH": str(tools) + os.pathsep + os.environ.get("PATH", "")},
                capture_output=True, timeout=15)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"Provider SDK resolution differs from shared contract", result.stderr)
            self.assertFalse(output.exists())

    def test_dangling_destination_link_refuses_before_provider_work(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = root / "observation.json"
            target.symlink_to(root / "absent-target")
            for stage in STAGES:
                with self.subTest(stage=stage):
                    result = subprocess.run(
                        [sys.executable, str(ROOT / "conformance/integration" / stage / "run.py"),
                         "--provider-root", str(root / "missing-provider"), "--output", str(target)],
                        cwd=root, capture_output=True, timeout=15)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(b"destination exists", result.stderr.lower())
                    self.assertTrue(target.is_symlink())
                    self.assertFalse((root / "absent-target").exists())

    def test_existing_observation_refuses_before_provider_work(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = root / "observation.json"
            original = b"retained prior execution\n"
            target.write_bytes(original)
            for stage in STAGES:
                with self.subTest(stage=stage):
                    result = subprocess.run(
                        [sys.executable, str(ROOT / "conformance/integration" / stage / "run.py"),
                         "--provider-root", str(root / "missing-provider"), "--output", str(target)],
                        cwd=root, capture_output=True, timeout=15)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(b"destination exists", result.stderr.lower())
                    self.assertEqual(target.read_bytes(), original)

    def test_complete_new_observation_and_late_collision(self):
        # Cargo/rustc are external facades here. These synthetic responses test
        # only the CLI's file publication, never real provider qualification.
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            provider = root / "provider"
            crate = provider / "lamu-openwarrant"
            fixtures = crate / "tests/fixtures"
            fixtures.mkdir(parents=True)
            for name, original in [("task.md", "conformance/sdk/source/task.md"),
                                   ("architecture.md", "conformance/sdk/document/adr.md")]:
                (fixtures / name).write_bytes((ROOT / original).read_bytes())
            pin = "4b626f23b3076ff7e764f08eeb15e8bdfa6af5cd"
            (crate / "Cargo.toml").write_text(
                '[dependencies.openwarrant-core]\ngit="https://github.com/Quitetall/OpenWarrant.git"\n'
                f'rev="{pin}"\n')
            for name in ["Cargo.toml", "Cargo.lock"]:
                (provider / name).write_text("synthetic output-publication fixture\n")
            tools = root / "tools"
            tools.mkdir()
            cargo = tools / "cargo"
            cargo.write_text("#!/usr/bin/env python3\n" + '''import json, os, sys
from pathlib import Path
if "--example" in sys.argv:
    if os.environ["OW_TEST_COLLIDE"] == "1":
        Path(os.environ["OW_TEST_DEST"]).write_bytes(b"arrived after preflight\\n")
    print(os.environ["OW_TEST_PROFILE"])
else:
    for name in json.loads(os.environ["OW_TEST_NAMES"]):
        print("test " + name + " ... ok")
''')
            rustc = tools / "rustc"
            rustc.write_text("#!/usr/bin/env python3\nprint('synthetic output-publication test runtime')\n")
            cargo.chmod(0o755)
            rustc.chmod(0o755)
            profile = {"id": "lamu.openwarrant.selection/1", "sdk_revision": pin,
                       "version": "synthetic-output-publication-test",
                       "contract_sha256": hashlib.sha256((ROOT / "docs/integrations/selection.md").read_bytes()).hexdigest()}
            names = ["governing_unknown_pointer_includes_exact_transitive_background_and_all_reasons",
                     "cycles_terminate_and_missing_members_and_conflicts_block",
                     "optional_absence_is_catalogued_but_required_missing_and_included_dependencies_block",
                     "raw_document_target_retains_whole_blob_and_missing_optional_is_not_included",
                     "limits_are_positive_before_traversal_even_without_conditions",
                     "competing_required_revisions_refuse_and_master_order_is_stable"]
            output = root / "output"
            output.mkdir()
            for collision in [False, True]:
                target = output / ("late.json" if collision else "new.json")
                env = {**os.environ, "PATH": str(tools) + os.pathsep + os.environ.get("PATH", ""),
                       "OW_TEST_COLLIDE": "1" if collision else "0", "OW_TEST_DEST": str(target),
                       "OW_TEST_PROFILE": json.dumps(profile), "OW_TEST_NAMES": json.dumps(names)}
                result = subprocess.run(
                    [sys.executable, str(ROOT / "conformance/integration/selection/run.py"),
                     "--provider-root", str(provider), "--output", str(target)],
                    cwd=root, env=env, capture_output=True, timeout=15)
                if collision:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(target.read_bytes(), b"arrived after preflight\n")
                else:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    value = json.loads(target.read_bytes())
                    self.assertEqual(value["format"], "openwarrant-selection-integration-observation/1")
                    self.assertEqual(value["profile"], profile)
            self.assertEqual(sorted(p.name for p in output.iterdir()), ["late.json", "new.json"])


if __name__ == "__main__":
    unittest.main()
