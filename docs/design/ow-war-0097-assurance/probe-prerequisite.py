#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Reproduce nested-prerequisite classification in a disposable gate fixture."""

import argparse
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--war", required=True, help="Explicit CLI executable to observe")
    args = parser.parse_args()
    war = pathlib.Path(args.war).resolve(strict=True)
    root = pathlib.Path(__file__).resolve().parents[3]
    out = pathlib.Path(__file__).resolve().parent / "observations"
    with tempfile.TemporaryDirectory(prefix="ow97-prerequisite-probe-") as temporary:
        fixture = pathlib.Path(temporary)
        init = subprocess.run(
            [str(war), "init", "--namespace", "PROBE", "--program",
             "Nested prerequisite control", "--json"],
            cwd=fixture, capture_output=True, text=True, check=False,
        )
        if init.returncode:
            raise RuntimeError("Fixture initialization failed: " + init.stdout + init.stderr)
        script = fixture / "unavailable.py"
        script.write_text('import sys\nprint("UNKNOWN: cargo prerequisite unavailable")\nsys.exit(127)\n')
        definition = (root / "docs/gates/ops.echo@1.0.0.yaml").read_text()
        if definition.count('argv: ["true"]') != 1:
            raise RuntimeError("Probe must replace exactly one argv")
        definition = definition.replace(
            'argv: ["true"]', "argv: " + json.dumps([sys.executable, str(script)]),
        )
        gates = fixture / "docs/gates"
        gates.mkdir(parents=True, exist_ok=True)
        (gates / "ops.echo@1.0.0.yaml").write_text(definition)
        argv = [str(war), "gate", "--gate", "ops.echo@1.0.0", "--run", "--json"]
        run = subprocess.run(argv, cwd=fixture, capture_output=True, text=True, check=False)
        report = json.loads(run.stdout)
        if not any(d["rule"] == "gate-run.fail" for d in report["diagnostics"]):
            raise RuntimeError("The reported classification has changed; inspect the report")
        observation = {
            "schema": "oh.war/doctor-gate-prerequisite-observation/v1",
            "kind": "synthetic_control",
            "checkout_revision": subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True,
            ).strip(),
            "cli_path": str(war),
            "cli_sha256": hashlib.sha256(war.read_bytes()).hexdigest(),
            "argv": argv,
            "fixture": "Scratch ops.echo definition with argv replaced by a Python child printing UNKNOWN and exiting 127. No production definition changed.",
            "child_output": "UNKNOWN: cargo prerequisite unavailable",
            "child_exit_code": 127,
            "runner_exit_code": run.returncode,
            "runner_report": report,
        }
        out.mkdir(parents=True, exist_ok=True)
        (out / "unavailable-prerequisite.json").write_text(json.dumps(observation, indent=2) + "\n")
        print("Observed gate-run.fail for unavailable nested prerequisite; no production records changed")


if __name__ == "__main__":
    main()
