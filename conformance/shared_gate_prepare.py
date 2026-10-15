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
    case = parser.add_mutually_exclusive_group()
    case.add_argument("--mutate-input", action="store_true")
    case.add_argument("--mutate-contract", action="store_true")
    case.add_argument("--mutate-gate", action="store_true")
    case.add_argument("--warrant-specific", action="store_true")
    case.add_argument("--fail-gate", action="store_true")
    case.add_argument("--different-orders", action="store_true")
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
        selected = ["IX-WAR-0001", "IX-WAR-0003"]

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
                  "p.write_text((p.read_text() if p.exists() else '')+'run'+chr(10)); "
                  "print('observation-'+str(len(p.read_text().splitlines())))")
        if args.mutate_input:
            script += (f"; p=Path({str(root / 'openwarrant.toml')!r}); "
                       "p.write_text(p.read_text()+chr(10)+'# changed during gate'+chr(10))")
        if args.mutate_contract:
            contract = root / "docs/warrants/IX-WAR-0001/atoms/10-intent.md"
            script += (f"; p=Path({str(contract)!r}); "
                       "p.write_text(p.read_text()+chr(10)+'Changed while gate runs.'+chr(10))")
        if args.mutate_gate:
            definition = gates / "software.repo.war-check@1.0.0.yaml"
            script += (f"; p=Path({str(definition)!r}); "
                       "p.write_text(chr(10).join('argv: '+__import__('json').dumps(('false',)) if "
                       "line.startswith('argv: ') else line for line in "
                       "p.read_text().splitlines())+chr(10))")
        if args.fail_gate:
            script += "; raise SystemExit(7)"
        gate = (repo / "docs/gates/ops.echo@1.0.0.yaml").read_text()
        gate = gate.replace('gate_id: "ops.echo"',
                            'gate_id: "software.repo.war-check"')
        assert 'argv: ["true"]' in gate
        argv = ["python3", "-c", script]
        if args.warrant_specific:
            argv.append("<warrant>")
        gate = gate.replace('argv: ["true"]', "argv: " + json.dumps(argv))
        gate += '\ninputs: ["openwarrant.toml"]\n'
        (gates / "software.repo.war-check@1.0.0.yaml").write_text(gate)
        if args.different_orders:
            (gates / "plant.second@1.0.0.yaml").write_text(gate.replace(
                'gate_id: "software.repo.war-check"', 'gate_id: "plant.second"'))
            key = "gate://software.repo.war-check@1.0.0"
            other = "gate://plant.second@1.0.0"
            for ordinal, alias in enumerate(selected):
                assurance = root / "docs/warrants" / alias / "atoms/60-assurance.md"
                text = assurance.read_text()
                assert text.count(key) == 2
                if ordinal == 0:
                    left, right = text.rsplit(key, 1)
                    text = left + other + right
                else:
                    text = text.replace(key, other, 1)
                assurance.write_text(text)
        compiled = cli("admin", "compile")
        assert compiled["exit_code"] == 0, compiled
        git("add", "-A")
        git("commit", "-qm", "synthetic shared-gate fixture")

        report = cli("evidence", "prepare", *selected, "--jobs", "1")
        results = report["result"]["warrants"]
        for alias in selected:
            warrant = next(w for w in results if w["alias"] == alias)
            steps = [s for s in warrant["steps"] if s["step"] == "evidence"]
            expected = ("unknown" if args.mutate_input or args.mutate_gate or (
                args.mutate_contract and alias == selected[0]) else
                "failed" if args.fail_gate else "done")
            assert len(steps) == (2 if args.different_orders else 1), warrant
            assert all(s["outcome"] == expected for s in steps), warrant
        observed = counter.read_text().splitlines()
        assert observed == ["run"] * (2 if args.warrant_specific or args.different_orders else 1), (
            f"one shared gate must execute once, observed {len(observed)} starts: {observed}"
        )
        if args.different_orders:
            for slug in ("software_repo_war-check_1_0_0", "plant_second_1_0_0"):
                bound = [json.loads((root / "docs/warrants" / alias / "gate-runs"
                                     / f"{slug}.receipt.json").read_text()) for alias in selected]
                assert bound[0] == bound[1]
                assert len({s for s in bound[0]["subject_digests"] if s.startswith("contract:")}) == 2
            print("PASS different gate orders: each gate executes once with both contracts pre-bound")
            return
        if args.mutate_input or args.mutate_gate:
            for alias in selected:
                assert not (root / "docs/warrants" / alias / "gate-runs"
                            / "software_repo_war-check_1_0_0.receipt.json").exists(), alias
            print("PASS changed shared question: UNKNOWN, no current attachments")
            return
        if args.mutate_contract:
            for alias in selected:
                attached = (root / "docs/warrants" / alias / "gate-runs"
                            / "software_repo_war-check_1_0_0.receipt.json").exists()
                assert attached == (alias != selected[0]), alias
            print("PASS changed contract: affected Warrant UNKNOWN; independent member continues")
            return
        receipts = [json.loads((root / "docs/warrants" / alias / "gate-runs"
                               / "software_repo_war-check_1_0_0.receipt.json").read_text())
                    for alias in selected]
        assert not (root / "docs/warrants/IX-WAR-0002/gate-runs"
                    / "software_repo_war-check_1_0_0.receipt.json").exists()
        if args.warrant_specific:
            contracts = [{s for s in r["subject_digests"] if s.startswith("contract:")}
                         for r in receipts]
            assert all(len(c) == 1 for c in contracts) and contracts[0].isdisjoint(contracts[1])
            assert receipts[0] != receipts[1]
            print("PASS Warrant-specific gate: two executions, separate contract bindings")
            return
        assert receipts[0] == receipts[1], "attachments must retain one actual sealed observation"
        contracts = {s for s in receipts[0]["subject_digests"] if s.startswith("contract:")}
        assert len(contracts) == 2, "both selected contracts must be bound before execution"
        if args.fail_gate:
            assert receipts[0]["verdict"] == "fail"
            assert all(w["state"] == "stopped" for w in results)
            print("PASS shared failure: one execution, both members stop, failed observation retained")
            return
        print("PASS shared preparation: one execution, two pre-bound contract attachments")
        streams = {name: (root / receipts[0][name]).read_bytes()
                   for name in ("stdout_ref", "stderr_ref")}
        journals = {alias: (root / "docs/warrants" / alias / "journal.jsonl").read_bytes()
                    for alias in selected}
        replay = cli("evidence", "prepare", *selected, "--jobs", "1")
        for warrant in replay["result"]["warrants"]:
            step = next(s for s in warrant["steps"] if s["step"] == "evidence")
            assert step["outcome"] == "current", warrant
        assert counter.read_text().splitlines() == ["run"], "equivalent retry executed again"
        for alias in selected:
            assert (root / "docs/warrants" / alias / "journal.jsonl").read_bytes() == journals[alias]
        print("PASS equivalent retry: no execution and no extra attachment event")
        config = root / "openwarrant.toml"
        config.write_text(config.read_text() + "\n# next source revision\n")
        git("add", "-A")
        git("commit", "-qm", "move the synthetic gate input")
        renewed = cli("evidence", "prepare", *selected, "--jobs", "1")
        for warrant in renewed["result"]["warrants"]:
            step = next(s for s in warrant["steps"] if s["step"] == "evidence")
            assert step["outcome"] == "done", warrant
        assert counter.read_text().splitlines() == ["run", "run"]
        for name, original in streams.items():
            assert (root / receipts[0][name]).read_bytes() == original, "earlier stream overwritten"
        latest = json.loads((root / "docs/warrants" / selected[0] / "gate-runs"
                             / "software_repo_war-check_1_0_0.receipt.json").read_text())
        assert latest["stdout_ref"] != receipts[0]["stdout_ref"]
        assert (root / latest["stdout_ref"]).read_bytes() != streams["stdout_ref"]
        print("PASS changed input reruns once and preserves the earlier observation streams")


if __name__ == "__main__":
    main()
