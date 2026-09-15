#!/usr/bin/env python3
"""Exercise the actual LAMU provider; retain exact inputs and bounded observations.

Run from any directory with --provider-root pointing to LAMU's Rust workspace.
This is a test driver, not an authority or provider attestation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(argv, root):
    completed = subprocess.run(argv, cwd=root, text=True, capture_output=True, timeout=600)
    return {"argv": argv, "exit_code": completed.returncode,
            "stdout": completed.stdout, "stderr": completed.stderr}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    root = args.provider_root.resolve(strict=True)
    ow = Path(__file__).resolve().parents[3]
    crate = root / "lamu-openwarrant"
    contract = ow / "docs/integrations/lamu-source-provider.md"
    manifest = tomllib.loads((crate / "Cargo.toml").read_text())
    dependency = manifest["dependencies"]["openwarrant-core"]
    expected_sdk = "8f048eee8c5679660e20537f82ddf1b923b21a3f"
    if dependency != {"git": "https://github.com/Quitetall/OpenWarrant.git", "rev": expected_sdk}:
        raise ValueError("Provider does not consume the agreed SDK revision")
    fixtures = [("task.md", "conformance/sdk/source/task.md"),
                ("architecture.md", "conformance/sdk/document/adr.md")]
    for name, original in fixtures:
        if (crate / "tests/fixtures" / name).read_bytes() != (ow / original).read_bytes():
            raise ValueError(f"Fixture differs from OpenWarrant source: {name}")
    profile_run = run(["cargo", "+1.97.1", "run", "--locked", "--quiet", "-p",
                       "lamu-openwarrant", "--example", "profile"], root)
    if profile_run["exit_code"]:
        raise RuntimeError(profile_run["stderr"])
    profile = json.loads(profile_run["stdout"])
    if profile["id"] != "lamu.openwarrant.local-source/1" or profile["sdk_revision"] != expected_sdk:
        raise ValueError("Unexpected provider profile")
    if profile["contract_sha256"] != digest(contract):
        raise ValueError("Provider contract digest mismatch")
    tests = run(["cargo", "+1.97.1", "test", "--locked", "-p", "lamu-openwarrant",
                 "--test", "source", "--", "--test-threads=1"], root)
    expected_tests = [
        "capture_original_bytes_and_refuse_changed_source_without_replacing_prior_snapshot",
        "resolve_units_from_actual_capture_preserves_old_basis_after_heading_rename",
        "mixed_sources_have_deterministic_locks_and_distinct_integrity_refusals",
        "filesystem_boundary_refuses_escape_symlinks_special_files_and_limits",
        "aggregate_budget_stops_before_next_filesystem_lookup",
    ]
    passed = tests["exit_code"] == 0 and all(
        f"test {name} ... ok" in tests["stdout"] for name in expected_tests)
    inputs = sorted(p for p in crate.rglob("*") if p.is_file()) + [root / "Cargo.lock", root / "Cargo.toml"]
    receipt = {
        "format": "openwarrant-source-integration-observation/1", "passed": passed,
        "profile": profile, "provider_git_head": run(["git", "rev-parse", "HEAD"], root)["stdout"].strip(),
        "provider_worktree_status": run(["git", "status", "--short"], root)["stdout"],
        "toolchain": run(["rustc", "+1.97.1", "--version"], root)["stdout"].strip(),
        "inputs_sha256": {str(p.relative_to(root)): digest(p) for p in inputs},
        "profile_process": profile_run, "tests": tests,
        "scope": "T11-T15 local Unix source profile plus aggregate-budget regression",
        "limitations": ["No human assurance", "No atomic cross-file filesystem snapshot",
                        "No dependency closure or context compiler", "Host OS only"],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"passed": passed, "receipt": str(args.output), "tests": len(expected_tests)}))
    return 0 if passed else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.TimeoutExpired) as exc:
        print(f"Integration refused: {exc}", file=sys.stderr)
        sys.exit(1)
