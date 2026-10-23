#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe condition syntax through the SDK CLI, without evaluating applicability."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import uuid


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    binary = args.war.resolve(strict=True)
    root = args.fixtures.resolve(strict=True)
    output = args.record_dir.resolve()
    if output == root or output.is_relative_to(root):
        raise ValueError("observations must be outside protected fixtures")
    output.mkdir(parents=True, exist_ok=False)
    suite_raw = (root / "cases.json").read_bytes()
    suite = json.loads(suite_raw)
    assert suite["schema"] == "oh.war/sdk-condition-cases/v1" and suite["cases"]
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}
    observations = []
    sources = {"cases.json": digest(suite_raw)}
    ids = set()
    with (output / "wire.jsonl").open("x", encoding="utf-8") as wire:
        def call(label, condition, expected_error=None, extra=()):
            request = {"schema": "oh.war/sdk-request/v1", "operation": "condition", "condition": condition}
            raw = json.dumps(request, ensure_ascii=False, separators=(",", ":")).encode()
            argv = [str(binary), "sdk", "--request", "-", *extra]
            p = subprocess.run(argv, input=raw, capture_output=True, env=env, timeout=30)
            wire.write(json.dumps({"label": label, "argv": argv, "stdin": raw.decode(),
                       "stdout": p.stdout.decode(), "stderr": p.stderr.decode(), "exit_code": p.returncode},
                       ensure_ascii=False, separators=(",", ":")) + "\n")
            wire.flush()
            value = json.loads(p.stdout)
            assert value["schema"] == "oh.war/report/v1" and value["command"] == "sdk"
            assert value["exit_code"] == p.returncode
            if expected_error is None:
                assert p.returncode == 0, value
                assert value["result"] == {"syntax": "valid", "applicability": "not-evaluated"}
            else:
                assert p.returncode != 0 and value["result"] is None, value
                assert any(d.get("code") == expected_error for d in value["diagnostics"]), value
            observations.append({"label": label, "expected_error": expected_error,
                                 "stdin_sha256": digest(raw), "stdout_sha256": digest(p.stdout)})
            return value

        for case in suite["cases"]:
            assert case["id"] not in ids
            ids.add(case["id"])
            path = (root / case["file"]).resolve(strict=True)
            if not path.is_relative_to(root):
                raise ValueError("fixture escapes its root")
            raw = path.read_bytes()
            sources[case["file"]] = digest(raw)
            call(case["id"], tomllib.loads(raw.decode()), case["error"])
            print("PASS", case["id"], flush=True)

        suffix = uuid.uuid4().hex
        unseen = {"stage": ["stage-" + suffix], "subsystem": ["subsystem-" + suffix],
                  "path": ["src/" + suffix + "/**", "é/*"]}
        call("unseen-valid", unseen)
        call("unseen-empty-stage", {**unseen, "stage": []}, "condition-invalid")
        with tempfile.TemporaryDirectory(prefix="ow-condition-output-", dir=output) as temp:
            target = Path(temp) / "result.json"
            saved = call("output-new", unseen, extra=("--output", str(target)))
            assert json.loads(target.read_bytes()) == saved
            before = target.read_bytes()
            call("output-existing", unseen, "sdk.output", ("--output", str(target)))
            assert target.read_bytes() == before

    result = {"kind": "actual-sdk-condition-observations", "cases": sorted(ids),
              "sdk_calls": len(observations), "calls": observations, "fixture_sha256": sources,
              "binary_sha256": digest(binary.read_bytes()),
              "wire_sha256": digest((output / "wire.jsonl").read_bytes()),
              "applicability_evaluated": False, "qualification_established": False}
    (output / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"PASS {len(ids)} condition cases and unseen/protected-output controls", flush=True)


if __name__ == "__main__":
    main()
