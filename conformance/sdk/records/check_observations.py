#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Record actual offline CLI evaluations; supplied facts never become signatures."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
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
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    war = args.war.resolve(strict=True)
    cases_path = args.cases.resolve(strict=True)
    output = args.record_dir.resolve()
    if output.is_relative_to(cases_path.parent.parent):
        raise ValueError("observations must be outside protected fixtures")
    output.mkdir(parents=True, exist_ok=False)
    fixture_raw = cases_path.read_bytes()
    suite = json.loads(fixture_raw)
    assert suite["schema"] == "oh.war/sdk-cli-cases/v1"
    operations = {"records", "readiness", "assurance", "agent-act", "workflow-decode",
                  "handoff", "replay", "resume", "availability", "batch-acceptance",
                  "schedule", "difficulty"}
    cases = {c["id"]: c for c in suite["cases"] if c["request"]["operation"] in operations}
    assert cases and len(cases) == sum(c["request"]["operation"] in operations for c in suite["cases"])
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}
    calls = []
    with (output / "sdk-wire.jsonl").open("xb") as wire:
        def call(label, request, success=True, error=None, expected=None, message=None):
            raw = json.dumps(request, ensure_ascii=False).encode()
            argv = [str(war), "sdk", "--request", "-"]
            process = subprocess.run(argv, input=raw, capture_output=True, timeout=30,
                                     cwd=output, env=env)
            wire.write((json.dumps({"label": label, "argv": argv, "stdin": raw.decode(),
                        "stdout": process.stdout.decode(), "stderr": process.stderr.decode(),
                        "exit_code": process.returncode}, ensure_ascii=False) + "\n").encode())
            wire.flush()
            value = json.loads(process.stdout)
            assert value["schema"] == "oh.war/report/v1", value
            assert value["exit_code"] == process.returncode, value
            assert (process.returncode == 0) == success, value
            if error:
                assert value["result"] is None, value
                assert any(d["code"] == error for d in value["diagnostics"]), value
                if message:
                    assert any(d["message"] == message for d in value["diagnostics"]), value
            for pointer, want in (expected or {}).items():
                got = value
                for key in pointer.lstrip("/").split("/"):
                    key = key.replace("~1", "/").replace("~0", "~")
                    got = got[int(key)] if isinstance(got, list) else got[key]
                assert got == want, (label, pointer, got, want)
            result = value["result"]
            if success and request["operation"] == "assurance":
                assert result["qualification_established"] is False
                assert result["assumptions_authenticated_by_cli"] is False
                assert result["evaluation"]["issues_mark"] is False
            calls.append({"label": label, "stdin_sha256": sha(raw),
                          "stdout_sha256": sha(process.stdout), "exit_code": process.returncode})
            print("PASS", label, flush=True)
            return result

        for label, case in cases.items():
            call(label, case["request"], case["success"], case.get("error"), case["expect"])

        # Fresh subjects ensure these observations are not fixture-name lookups.
        schedule = copy.deepcopy(cases["schedule"]["request"])
        nonce = uuid.uuid4().hex
        for i, scope in enumerate(schedule["scopes"]):
            scope["warrant"] = f"observation:{nonce}:{i}"
            scope["stage"] = f"stage-{i}"
        fact = {"scope": copy.deepcopy(schedule["scopes"][0]), "requirement": "implementation-finished",
                "result_digest": "sha256:" + "d" * 64, "state": "satisfied"}
        schedule["facts"] = [fact]
        result = call("fresh-matching-result", schedule)
        assert result["evaluation"]["ready"] == [0, 1]
        assert result["dispatch_permitted"] is False
        for field, replacement in [("contract_digest", "sha256:" + "e" * 64),
                                   ("stage", "different-stage")]:
            stale = copy.deepcopy(schedule)
            stale["facts"][0]["scope"][field] = replacement
            result = call("fresh-stale-" + field, stale)
            assert result["evaluation"]["ready"] == [0]
            assert result["evaluation"]["waiting"][0]["state"] == "unknown"
        ambiguous = copy.deepcopy(schedule)
        second = copy.deepcopy(fact)
        second["result_digest"] = "sha256:" + "f" * 64
        ambiguous["facts"].append(second)
        call("fresh-ambiguous-output", ambiguous, False, "sdk.schedule", message="AmbiguousResult")
        ambiguous["dependencies"][0]["result_digest"] = fact["result_digest"]
        result = call("fresh-pinned-output", ambiguous)
        assert result["evaluation"]["ready"] == [0, 1]

        readiness = copy.deepcopy(cases["readiness-unknown"]["request"])
        readiness["conditions"][0]["purpose"] = "qualification"
        result = call("qualification-does-not-block-action", readiness)
        assert result["evaluation"]["action_ready"] is True
        assert result["evaluation"]["qualification"][0]["state"] == "unknown"
        readiness["conditions"][0]["purpose"] = "action-gate"
        result = call("explicit-action-gate-blocks-action", readiness)
        assert result["evaluation"]["action_ready"] is False
        readiness["stage"] = "unrelated-stage"
        result = call("gate-is-scoped-to-stage", readiness)
        assert result["evaluation"]["action_ready"] is True

    assert cases_path.read_bytes() == fixture_raw
    receipt = {"schema": "oh.war/records-cli-observation/v1", "binary_sha256": sha(war.read_bytes()),
               "fixture_sha256": sha(fixture_raw), "fixture_cases": len(cases), "calls": calls,
               "qualification_established": False, "scope": "Offline supplied-record evaluation only; no authentication, dispatch, human acceptance or mark"}
    publish_observation(output / "observation.json", receipt)
    print(f"{len(calls)} observed calls; no qualification issued", flush=True)


if __name__ == "__main__":
    main()
