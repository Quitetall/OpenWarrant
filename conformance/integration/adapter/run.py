#!/usr/bin/env python3
"""Exercise real LAMU compiler plus an independent SDK-only package consumer."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib


def run(argv, cwd, data=None):
    result = subprocess.run(argv, cwd=cwd, input=data, text=True,
                            capture_output=True, timeout=600)
    return {"argv": argv, "exit_code": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root = args.provider_root.resolve(strict=True)
    ow = Path(__file__).resolve().parents[3]
    crate = root / "lamu-openwarrant"
    pin = "4b626f23b3076ff7e764f08eeb15e8bdfa6af5cd"
    dep = tomllib.loads((crate / "Cargo.toml").read_text())["dependencies"]["openwarrant-core"]
    if dep != {"git": "https://github.com/Quitetall/OpenWarrant.git", "rev": pin}:
        raise ValueError("Provider SDK pin differs from shared contract")
    head = run(["git", "rev-parse", "HEAD"], root)["stdout"].strip()
    observations = {}
    with tempfile.TemporaryDirectory(prefix="ow86-integration-") as tmp:
        for stage in ["source", "conditions", "selection", "projections", "package", "budgets"]:
            destination = Path(tmp) / f"{stage}.json"
            process = run([sys.executable, str(ow / f"conformance/integration/{stage}/run.py"),
                           "--provider-root", str(root), "--output", str(destination)], ow)
            if process["exit_code"]:
                raise RuntimeError(f"{stage}: {process['stderr']}")
            observations[stage] = json.loads(destination.read_text())
    tests = run(["cargo", "+1.97.1", "test", "--locked", "-p", "lamu-openwarrant",
                 "--test", "adapter", "--test", "adapter_allocations"], root)
    command = ["cargo", "+1.97.1", "run", "--locked", "--quiet", "-p", "lamu-openwarrant",
               "--example", "compile_package", "--", str(ow / "conformance/sdk/source"),
               "task.md", f"lamu-openwarrant@{head}"]
    generated = run(command, root)
    repeated = run(command, root)
    if generated["exit_code"] or repeated["exit_code"]:
        raise RuntimeError(generated["stderr"] + repeated["stderr"])
    payload = generated["stdout"]
    consumer = ["cargo", "+1.97.1", "run", "--locked", "--quiet", "-p",
                "openwarrant-core", "--example", "audit_context_package"]
    accepted = run(consumer, ow, payload)
    bad = json.loads(payload)
    bad["ENTRY.md"].append(120)
    tampered = run(consumer, ow, json.dumps(bad))
    duplicate = run(consumer, ow, '{"ENTRY.md":[],' + payload.lstrip()[1:])
    audit = json.loads(accepted["stdout"]) if accepted["exit_code"] == 0 else {}
    passed = (tests["exit_code"] == 0 and all(o["passed"] for o in observations.values())
              and repeated["stdout"] == payload and accepted["exit_code"] == 0
              and audit.get("local_integrity") is True
              and audit.get("local_semantic_coverage") is False
              and audit.get("readiness") == "blocked"
              and tampered["exit_code"] != 0 and "digest-mismatch" in tampered["stderr"]
              and duplicate["exit_code"] != 0 and "duplicate" in duplicate["stderr"])
    receipt = {"format": "openwarrant-adapter-integration-observation/1", "passed": passed,
               "provider_git_head": head, "sdk_revision": pin,
               "provider_worktree_status": run(["git", "status", "--short"], root)["stdout"],
               "shared_contract_sha256": hashlib.sha256((ow / "docs/integrations/sdk.md").read_bytes()).hexdigest(),
               "provider_cases": observations, "adapter_tests": tests,
               "producer_argv": command, "package_map_sha256": hashlib.sha256(payload.encode()).hexdigest(),
               "repeat_identical": repeated["stdout"] == payload,
               "second_consumer": accepted, "tamper_refusal": tampered,
               "duplicate_refusal": duplicate,
               "limitations": ["No human assurance", "Task readiness evaluation remains blocked",
                               "Phase 1 exit and complete record/legacy exchange remain pending",
                               "This does not declare Phase 2 exit"]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"passed": passed, "receipt": str(args.output)}))
    return 0 if passed else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.TimeoutExpired) as exc:
        print(f"Integration refused: {exc}", file=sys.stderr)
        sys.exit(1)
