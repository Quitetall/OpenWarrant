#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Exercise shared preparation through the public CLI on a disposable corpus.

Fixture authorization and gate output are synthetic. They establish recorder
behavior only, never independent verification or authority in a real project.
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--war", required=True, type=Path)
    args = parser.parse_args()
    war = str(args.war.resolve(strict=True))
    repo = Path(__file__).resolve().parent.parent
    env = dict(os.environ)
    for name in ("SSH_AUTH_SOCK", "SSH_AGENT_PID", "GIT_DIR", "GIT_WORK_TREE",
                 "GIT_INDEX_FILE", "GIT_COMMON_DIR"):
        env.pop(name, None)
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null",
               OPENWARRANT_NO_PROJECTS="1", OPENWARRANT_NO_UPDATE_CHECK="1",
               GIT_AUTHOR_NAME="fixture", GIT_AUTHOR_EMAIL="fixture@invalid",
               GIT_COMMITTER_NAME="fixture", GIT_COMMITTER_EMAIL="fixture@invalid")
    with tempfile.TemporaryDirectory(prefix="ow-shared-gate-") as temporary:
        root = Path(temporary) / "repository"
        shutil.copytree(repo / "conformance/fixtures/inbox/repository", root)

        def git(*argv):
            return subprocess.run(["git", *argv], cwd=root, env=env,
                                  capture_output=True, check=True, timeout=30)

        def cli(*argv):
            result = subprocess.run([war, *argv, "--json"], cwd=root, env=env,
                                    capture_output=True, timeout=90)
            try:
                return json.loads(result.stdout)
            except ValueError as error:
                raise AssertionError(result.stderr.decode(errors="replace")) from error

        git("init", "-q")
        gates = root / "docs/gates"
        gates.mkdir(exist_ok=True)
        counter = root / ".git/gate-starts"
        # Count real child starts outside source inputs, without mocking any
        # recorder or depending on its private implementation.
        script = ("from pathlib import Path; "
                  f"p=Path({str(counter)!r}); "
                  "p.write_text((p.read_text() if p.exists() else '')+'run'+chr(10))")
        gate = (repo / "docs/gates/ops.echo@1.0.0.yaml").read_text()
        gate = gate.replace('gate_id: "ops.echo"',
                            'gate_id: "software.repo.war-check"')
        assert 'argv: ["true"]' in gate
        gate = gate.replace('argv: ["true"]',
                            "argv: " + json.dumps(["python3", "-c", script]))
        gate += '\ninputs: ["openwarrant.toml"]\n'
        (gates / "software.repo.war-check@1.0.0.yaml").write_text(gate)
        compiled = cli("admin", "compile")
        assert compiled["exit_code"] == 0, compiled
        git("add", "-A")
        git("commit", "-qm", "synthetic shared-gate fixture")

        selected = ["IX-WAR-0001", "IX-WAR-0003"]
        report = cli("evidence", "prepare", *selected, "--jobs", "1")
        results = report["result"]["warrants"]
        for alias in selected:
            warrant = next(w for w in results if w["alias"] == alias)
            steps = [s for s in warrant["steps"] if s["step"] == "evidence"]
            assert len(steps) == 1 and steps[0]["outcome"] == "done", warrant
        observed = counter.read_text().splitlines()
        assert observed == ["run"], (
            f"one shared gate must execute once, observed {len(observed)} starts: {observed}"
        )
        receipts = [json.loads((root / "docs/warrants" / alias / "gate-runs"
                               / "software_repo_war-check_1_0_0.receipt.json").read_text())
                    for alias in selected]
        assert receipts[0] == receipts[1], "attachments must retain one actual sealed observation"
        contracts = {s for s in receipts[0]["subject_digests"] if s.startswith("contract:")}
        assert len(contracts) == 2, "both selected contracts must be bound before execution"
        print("PASS shared preparation: one execution, two pre-bound contract attachments")


if __name__ == "__main__":
    main()
