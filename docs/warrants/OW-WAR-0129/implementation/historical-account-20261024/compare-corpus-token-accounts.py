#!/usr/bin/env python3
"""Compare trusted historical CLIs on a disposable clone of the named corpus."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

REVISION = "4f1f6a80fd65c5a0c0e7b256a1fc494f43248a79"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    tools = {name: getattr(args, name).resolve(strict=True) for name in ("before", "after")}
    repo = args.repo.resolve(strict=True)
    scratch, out = args.scratch, args.out
    if not scratch.is_absolute() or not out.is_absolute():
        parser.error("scratch and out must be absolute")
    if scratch == out or scratch in out.parents or out in scratch.parents:
        parser.error("scratch and out must not contain one another")
    out.mkdir(exist_ok=False)
    scratch.mkdir(exist_ok=False)
    env = dict(os.environ, OPENWARRANT_NO_CACHE="1", OPENWARRANT_NO_PROJECTS="1",
               OPENWARRANT_NO_UPDATE_CHECK="1")
    for name in list(env):
        if name in {"GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR",
                    "GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES",
                    "GIT_NAMESPACE", "GIT_PREFIX"} or name.startswith("GIT_CONFIG_"):
            env.pop(name)
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
    commands, accounts, source_witnesses = [], {}, {}

    def run(name, argv):
        argv = list(map(str, argv))
        if argv[0] == "git":
            argv[1:1] = ["-c", "core.hooksPath=" + os.devnull]
        result = subprocess.run(argv, cwd=scratch, env=env, capture_output=True, timeout=60)
        (out / (name + ".stdout")).write_bytes(result.stdout)
        (out / (name + ".stderr")).write_bytes(result.stderr)
        commands.append({"name": name, "argv": argv, "exit": result.returncode})
        if result.returncode:
            raise RuntimeError(f"{name}: exit {result.returncode}; inspect retained logs")
        return result.stdout

    try:
        run("clone", ["git", "clone", "--quiet", "--shared", "--no-hardlinks", repo, "."])
        run("checkout", ["git", "checkout", "--quiet", "--detach", REVISION])
        for label, tool in tools.items():
            run(label + "-reset", ["git", "reset", "--hard", REVISION])
            run(label + "-clean", ["git", "clean", "-fd"])
            source_witnesses[label] = run(label + "-tree", ["git", "rev-parse", "HEAD^{tree}"]).decode().strip()
            packet, context = out / (label + ".json"), out / (label + "-context.json")
            run(label + "-dispatch", [tool, "--root", scratch, "--json", "dispatch",
                "OW-WAR-0047", "STAGE-002", "--prototype", "--emit", packet,
                "--emit-context", context])
            accounts[label] = json.loads(packet.read_bytes())["tokens"]
        before_context, after_context = (out / "before-context.json").read_bytes(), (out / "after-context.json").read_bytes()
        if not accounts["before"] or accounts["before"] != accounts["after"]:
            raise RuntimeError("token accounts differ or are absent")
        if json.loads(before_context) != json.loads(after_context):
            raise RuntimeError("selected context manifests differ")
        if source_witnesses["before"] != source_witnesses["after"]:
            raise RuntimeError("input corpus trees differ")
        result = {"corpus_source": REVISION, "warrant": "OW-WAR-0047", "stage": "STAGE-002",
            "mode": "prototype compilation only; no actor execution", "accounts": accounts,
            "equal": True, "context_manifests_identical": True,
            "context_sha256": hashlib.sha256(before_context).hexdigest(),
            "input_trees": source_witnesses, "commands": commands, "qualification": False}
        (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(accounts))
    finally:
        (out / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        shutil.rmtree(scratch)


if __name__ == "__main__":
    main()
