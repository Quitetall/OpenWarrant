#!/usr/bin/env python3
"""Exercise real LAMU compiler plus an independent SDK-only package consumer."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
from exchange import exchange


def run(argv, cwd, data=None):
    if argv[0] == "cargo" and "lamu-openwarrant" in argv:
        index = argv.index("-p")
        argv = argv[:index] + ["--manifest-path", str(cwd / "lamu-openwarrant/Cargo.toml")] + argv[index:]
    result = subprocess.run(argv, cwd=cwd, input=data, text=True,
                            capture_output=True, timeout=600)
    return {"argv": argv, "exit_code": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def inputs(ow, provider):
    result={}
    for label,root,folders in [('sdk',ow,['crates','conformance/sdk','conformance/integration','docs/integrations']),
                               ('provider',provider,['lamu-openwarrant'])]:
        paths=[]
        for folder in folders:
            for directory,dirs,names in os.walk(root/folder):
                dirs[:]=[d for d in dirs if d not in {'target','.git','__pycache__'}]
                if any((Path(directory)/d).is_symlink() for d in dirs):
                    raise ValueError('Symlink input directory requires explicit capture')
                paths.extend(Path(directory)/name for name in names if not name.endswith('.pyc'))
        paths += [root/name for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','rust-toolchain'] if (root/name).is_file()]
        if (root/'.cargo').is_dir(): paths += [p for p in (root/'.cargo').rglob('*') if p.is_file()]
        for path in paths:
            if path.is_symlink():raise ValueError('Symlink build input requires explicit capture: '+str(path))
            result[label+'/'+path.relative_to(root).as_posix()]=hashlib.sha256(path.read_bytes()).hexdigest()
    return dict(sorted(result.items()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("Receipt destination exists")
    root = args.provider_root.resolve(strict=True)
    ow = Path(__file__).resolve().parents[3]
    crate = root / "lamu-openwarrant"
    pin = "4b626f23b3076ff7e764f08eeb15e8bdfa6af5cd"
    dep = tomllib.loads((crate / "Cargo.toml").read_text())["dependencies"]["openwarrant-core"]
    if dep != {"git": "https://github.com/Quitetall/OpenWarrant.git", "rev": pin}:
        raise ValueError("Provider SDK pin differs from shared contract")
    head = run(["git", "rev-parse", "HEAD"], root)["stdout"].strip()
    before = inputs(ow, root)
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
    build = run(["cargo", "+1.97.1", "build", "--locked", "-p", "openwarrant-cli",
                 "--bin", "war", "--message-format=json"], ow)
    if build["exit_code"]:raise RuntimeError(build["stderr"])
    artifacts = {item["executable"] for line in build["stdout"].splitlines()
                 if (item := json.loads(line)).get("reason") == "compiler-artifact"
                 and item.get("target", {}).get("name") == "war" and item.get("executable")}
    if len(artifacts) != 1:raise ValueError("CLI build identity missing")
    war = Path(artifacts.pop()).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="ow86-exchange-") as tmp:
        exchanged = exchange(ow, war, payload, audit, run, command, root, tmp)
    after = inputs(ow, root)
    changed = sorted(k for k in before.keys() | after.keys() if before.get(k) != after.get(k))
    passed = (not changed and tests["exit_code"] == 0 and exchanged["passed"] and all(o["passed"] for o in observations.values())
              and repeated["stdout"] == payload and accepted["exit_code"] == 0
              and audit.get("local_integrity") is True
              and audit.get("local_semantic_coverage") is False
              and audit.get("readiness") == "blocked"
              and tampered["exit_code"] != 0 and "digest-mismatch" in tampered["stderr"]
              and duplicate["exit_code"] != 0 and "duplicate" in duplicate["stderr"])
    receipt = {"format": "openwarrant-adapter-integration-observation/1", "passed": passed,
               "provider_git_head": head, "sdk_revision": pin,
               "consumer_git_head":run(["git","rev-parse","HEAD"],ow)["stdout"].strip(),
               "inputs_sha256":before,"changed_inputs":changed,
               "toolchain":run(["rustc","+1.97.1","-Vv"],ow)["stdout"],
               "provider_worktree_status": run(["git", "status", "--short"], root)["stdout"],
               "shared_contract_sha256": hashlib.sha256((ow / "docs/integrations/sdk.md").read_bytes()).hexdigest(),
               "provider_cases": observations, "adapter_tests": tests,
               "producer_argv": generated["argv"], "package_map_sha256": hashlib.sha256(payload.encode()).hexdigest(),
               "repeat_identical": repeated["stdout"] == payload,
               "second_consumer": accepted, "tamper_refusal": tampered,
               "duplicate_refusal": duplicate, "record_legacy_exchange": exchanged,
               "consumer_build": build,"consumer_binary_sha256":hashlib.sha256(war.read_bytes()).hexdigest(),
               "limitations": ["No human assurance", "Task readiness evaluation remains blocked",
                               "Native macOS Phase 1 evidence remains pending",
                               "This does not declare Phase 2 exit"]}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as output:
        json.dump(receipt, output, indent=2); output.write("\n")
    print(json.dumps({"passed": passed, "receipt": str(args.output)}))
    return 0 if passed else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.TimeoutExpired) as exc:
        print(f"Integration refused: {exc}", file=sys.stderr)
        sys.exit(1)
