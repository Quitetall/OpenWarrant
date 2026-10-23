#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe preservation through the public CLI, including exact historical Git bytes."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "integration"))
from observation import publish_observation


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--historical-revision", required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.historical_revision):
        raise ValueError("use the full historical Git commit id")
    war = args.war.resolve(strict=True)
    fixtures = args.fixtures.resolve(strict=True)
    repo = args.repo.resolve(strict=True)
    output = args.record_dir.resolve()
    if output.is_relative_to(fixtures):
        raise ValueError("observations must be outside protected fixtures")
    output.mkdir(parents=True, exist_ok=False)
    inputs = {}
    calls = []
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}

    def read(name):
        path = (fixtures / name).resolve(strict=True)
        if not path.is_relative_to(fixtures):
            raise ValueError("fixture path escapes root")
        raw = path.read_bytes()
        inputs[name] = sha(raw)
        return raw

    with (output / "sdk-wire.jsonl").open("xb") as wire:
        def call(label, operation, error=None, **fields):
            raw = json.dumps({"schema": "oh.war/sdk-request/v1", "operation": operation,
                              **fields}, ensure_ascii=False).encode()
            argv = [str(war), "sdk", "--request", "-"]
            process = subprocess.run(argv, input=raw, capture_output=True, timeout=30,
                                     cwd=output, env=env)
            wire.write((json.dumps({"label": label, "argv": argv, "stdin": raw.decode(),
                        "stdout": process.stdout.decode(), "stderr": process.stderr.decode(),
                        "exit_code": process.returncode}, ensure_ascii=False) + "\n").encode())
            wire.flush()
            value = json.loads(process.stdout)
            assert value["schema"] == "oh.war/report/v1" and value["exit_code"] == process.returncode
            if error:
                assert process.returncode != 0 and value["result"] is None, value
                assert any(d["code"] == error for d in value["diagnostics"]), value
            else:
                assert process.returncode == 0, value
                if operation in {"legacy-import", "legacy-capture"}:
                    assert value["result"]["qualification_established"] is False
                    assert value["result"]["historical_closure_established"] is False
                if operation == "successor":
                    assert value["result"]["authority_transferred"] is False
            calls.append({"label": label, "stdin_sha256": sha(raw),
                          "stdout_sha256": sha(process.stdout), "exit_code": process.returncode})
            print("PASS", label, flush=True)
            return value["result"]

        suite = json.loads(read("cases.json"))
        assert suite["schema"] == "oh.war/sdk-legacy-cases/v1"
        ids = set()
        for case in suite["cases"]:
            assert case["id"] not in ids
            ids.add(case["id"])
            raw = read(case["file"]).decode()
            if case.get("successor"):
                call(case["id"], "successor", case.get("error"), predecessor=raw,
                     successor=read(case["successor"]).decode(), paths=case["paths"])
            else:
                result = call(case["id"], "legacy-import", case.get("error"), payload=raw)
                if result:
                    restored = call(case["id"] + ":roundtrip", "legacy-import", payload=result["payload"])
                    assert restored["payload"] == result["payload"]
                    assert json.loads(restored["payload"])["files"] == json.loads(raw)["files"]

        # Check the fixture against Git, then pass Git's actual bytes to the SDK.
        historical = json.loads(read("resolved.json"))
        git_files = {}
        git_observations = []
        for path, stored in historical["files"].items():
            argv = ["git", "-C", str(repo), "show", args.historical_revision + ":" + path]
            process = subprocess.run(argv, capture_output=True, timeout=15)
            assert process.returncode == 0, process.stderr.decode()
            assert process.stdout == bytes(stored), path
            git_files[path] = process.stdout
            git_observations.append({"path": path, "argv": argv, "exit_code": process.returncode,
                                     "sha256": sha(process.stdout), "bytes": len(process.stdout)})
        result = call("historical-git-capture", "legacy-capture", dialect=historical["source_dialect"],
                      adapter=historical["adapter_version"], entry=historical["entry"],
                      files=[{"path": path, "hex": raw.hex()} for path, raw in git_files.items()])
        saved = json.loads(result["payload"])
        assert saved["files"] == historical["files"]
        assert result["original_state"] == "resolved-record-present"
        signature = next(path for path in git_files if path.endswith(".dsse.json"))
        damaged = copy.deepcopy(saved)
        damaged["files"][signature][0] ^= 1
        call("historical-signature-byte-tamper", "legacy-import", "legacy.integrity",
             payload=json.dumps(damaged))
        missing = copy.deepcopy(saved)
        del missing["files"][signature]
        call("historical-signature-file-missing", "legacy-import", "legacy.inventory",
             payload=json.dumps(missing))

        source = (fixtures.parent / "document/minimal-rc3.md").read_bytes()
        inputs["../document/minimal-rc3.md"] = sha(source)
        bundles = []
        for i in range(2):
            text = source.replace(b"example:minimal", ("observation:" + uuid.uuid4().hex).encode())
            assert text != source
            files = {"warrant.md": text, "src/lib.rs": f"// version {i}\r\n// 雪\n".encode(),
                     "archive/authority/original.dsse.json": git_files[signature],
                     "binary/original.bin": b"\x00\xff\r\n"}
            value = call(f"fresh-capture-{i}", "legacy-capture", dialect="rc3",
                         adapter=historical["adapter_version"], entry="warrant.md",
                         files=[{"path": path, "hex": raw.hex()} for path, raw in files.items()])
            assert json.loads(value["payload"])["files"] == {p: list(b) for p, b in files.items()}
            bundles.append(value["payload"])
        lineage = call("fresh-successor", "successor", predecessor=bundles[0],
                       successor=bundles[1], paths=["src/lib.rs"])
        assert lineage["versions"][0]["predecessor_digest"] != lineage["versions"][0]["successor_digest"]
        call("fresh-self-successor", "successor", "legacy.lineage",
             predecessor=bundles[0], successor=bundles[0], paths=["src/lib.rs"])
        call("fresh-unchanged-authority-path", "successor", "legacy.lineage",
             predecessor=bundles[0], successor=bundles[1], paths=["archive/authority/original.dsse.json"])
        call("fresh-duplicate-successor-path", "successor", "legacy.lineage",
             predecessor=bundles[0], successor=bundles[1], paths=["src/lib.rs", "src/lib.rs"])
        call("fresh-missing-successor-path", "successor", "legacy.missing",
             predecessor=bundles[0], successor=bundles[1], paths=["not-present"])
        call("fresh-traversal", "legacy-capture", "legacy.path", dialect="rc3",
             adapter=historical["adapter_version"], entry="../warrant.md",
             files=[{"path": "../warrant.md", "hex": source.hex()}])

    for name, digest in inputs.items():
        assert sha((fixtures / name).read_bytes()) == digest, name
    receipt = {"schema": "oh.war/legacy-cli-observation/v1", "binary_sha256": sha(war.read_bytes()),
               "fixture_sha256": inputs, "historical_revision": args.historical_revision,
               "historical_files": git_observations, "calls": calls,
               "qualification_established": False, "authority_transferred": False,
               "scope": "Exact supplied inventory and explicit successor paths only; no signature authentication or inferred historical closure"}
    publish_observation(output / "observation.json", receipt)
    print(f"{len(calls)} observed calls; {len(git_files)} exact Git files; no qualification issued", flush=True)


if __name__ == "__main__":
    main()
