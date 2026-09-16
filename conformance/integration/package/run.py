#!/usr/bin/env python3
"""Exercise the actual LAMU provider; retain exact inputs and bounded observations.

Run from any directory with --provider-root pointing to LAMU's Rust workspace.
This is a test driver, not an authority or provider attestation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(argv, root):
    if argv[0] == "cargo" and "lamu-openwarrant" in argv:
        index = argv.index("-p")
        argv = argv[:index] + ["--manifest-path", str(root / "lamu-openwarrant/Cargo.toml")] + argv[index:]
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
    contract = ow / "docs/integrations/package.md"
    manifest = tomllib.loads((crate / "Cargo.toml").read_text())
    dependency = manifest["dependencies"]["openwarrant-core"]
    expected_sdk = "4b626f23b3076ff7e764f08eeb15e8bdfa6af5cd"
    if dependency != {"git": "https://github.com/Quitetall/OpenWarrant.git", "rev": expected_sdk}:
        raise ValueError("Provider does not consume the agreed SDK revision")
    fixtures = [("task.md", "conformance/sdk/source/task.md"),
                ("architecture.md", "conformance/sdk/document/adr.md")]
    for name, original in fixtures:
        if (crate / "tests/fixtures" / name).read_bytes() != (ow / original).read_bytes():
            raise ValueError(f"Fixture differs from OpenWarrant source: {name}")
    profile_run = run(["cargo", "+1.97.1", "run", "--locked", "--quiet", "-p",
                       "lamu-openwarrant", "--example", "package_profile"], root)
    if profile_run["exit_code"]:
        raise RuntimeError(profile_run["stderr"])
    profile = json.loads(profile_run["stdout"])
    if profile["id"] != "lamu.openwarrant.package/1" or profile["sdk_revision"] != expected_sdk:
        raise ValueError("Unexpected provider profile")
    if profile["contract_sha256"] != digest(contract):
        raise ValueError("Provider contract digest mismatch")
    tests = run(["cargo", "+1.97.1", "test", "--locked", "-p", "lamu-openwarrant",
                 "--test", "package", "--test", "package_optional", "--test", "package_allocations", "--", "--test-threads=1"], root)
    expected_tests = [
        "offline_package_rebuilds_exact_views_and_refuses_tampering",
        "optional_opaque_absent_offline",
        "oversized_basis_refuses_before_copying",
    ]
    passed = tests["exit_code"] == 0 and all(
        f"test {name} ... ok" in tests["stdout"] for name in expected_tests)
    inputs = []
    for directory, dirs, names in os.walk(crate):
        dirs[:] = [d for d in dirs if d not in {"target", ".git"}]
        inputs.extend(Path(directory) / name for name in names)
    inputs = sorted(inputs) + [root / "Cargo.lock", root / "Cargo.toml"]
    receipt = {
        "format": "openwarrant-package-integration-observation/1", "passed": passed,
        "profile": profile, "provider_git_head": run(["git", "rev-parse", "HEAD"], root)["stdout"].strip(),
        "provider_worktree_status": run(["git", "status", "--short"], root)["stdout"],
        "toolchain": run(["rustc", "+1.97.1", "--version"], root)["stdout"].strip(),
        "inputs_sha256": {str(p.relative_to(root)): digest(p) for p in inputs},
        "profile_process": profile_run, "tests": tests,
        "scope": "T32-T37 portable package construction, semantic reconstruction and filesystem refusals",
        "limitations": ["No human assurance", "Task readiness blocked pending record evaluation",
                        "Host OS only"],
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
