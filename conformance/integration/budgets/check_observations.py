#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Consume external cache/budget observations without implementing a cache."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from observation import publish_observation


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--transcript", type=Path, required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    war = args.war.resolve(strict=True)
    raw = args.transcript.read_bytes()
    rows = [json.loads(line.split("OWOBS:", 1)[1]) for line in raw.decode().splitlines() if "OWOBS:" in line]
    cases = {row["case"]: row["observation"] for row in rows}
    assert len(rows) == len(cases) == 31
    assert all(row["qualification_established"] is False for row in rows)
    out = args.record_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}
    binary = sha(war.read_bytes())
    calls = []
    with (out / "sdk-wire.jsonl").open("xb") as wire:
        def sdk(label, operation, **fields):
            payload = json.dumps({"schema": "oh.war/sdk-request/v1", "operation": operation, **fields}).encode()
            p = subprocess.run([str(war), "sdk", "--request", "-"], input=payload, capture_output=True, env=env, timeout=30)
            wire.write((json.dumps({"label": label, "stdin": payload.decode(), "stdout": p.stdout.decode(),
                       "stderr": p.stderr.decode(), "exit_code": p.returncode}, ensure_ascii=False, separators=(",", ":")) + "\n").encode())
            wire.flush()
            value = json.loads(p.stdout)
            assert p.returncode == value["exit_code"] == 0, value
            calls.append({"label": label, "stdin_sha256": sha(payload), "stdout_sha256": sha(p.stdout)})
            return value["result"]

        keyed = ["baseline", "reactivated-rule", "source-revision"] + ["basis-" + name for name in
                 ["policy_facts", "records", "compiler", "role", "access-basis", "optional"]]
        keys = {}
        for label in keyed:
            case = cases[label]
            keys[label] = sdk(label, "digest", domain="basis", payload=case["request"])["digest"]
            assert keys[label] == case["basis_key"]
            if label != "baseline":
                assert keys[label] != keys["baseline"] and case["hit"] is False
        baseline = cases["baseline"]
        decoded = sdk("baseline-packet", "packet-decode", payload=baseline["packet_wire"])
        assert decoded["integrity_checked"] is False and decoded["semantic_coverage_established"] is False
        assert baseline["repeat_hit"] is True and baseline["same_arc"] is True
        original_limits = baseline["operational_limits"]
        names = ["json", "entry", "units", "edges", "reasons", "output", "working", "visits", "items", "strings", "steps"]
        for index, name in enumerate(names):
            case = cases["limit-" + name]
            changed = [i for i, (a, b) in enumerate(zip(original_limits, case["operational_limits"])) if a != b]
            assert changed == [index]
            assert case["operational_limits"][index] == original_limits[index] - 1
            assert case["hit"] is False and case["repeat_hit"] is True
        for size, tokens in [(0, 0), (1, 1), (3, 1), (4, 1), (5, 2), (1001, 251)]:
            case = cases["account-" + str(size)]
            assert case["bytes"] == size and case["estimated_tokens"] == case["budget"] == tokens
            assert case["below_error"] == ("budget-exceeded" if tokens else None)
        assert cases["unicode-account"] == {"bytes": 5, "estimated_tokens": 2}
        exact = cases["inclusive-final-entry"]
        assert exact["entry_bytes"] == len(baseline["entry"])
        assert exact["exact_request"]["options"]["entry_token_budget"] == exact["estimated_tokens"]
        assert exact["below_request"]["options"]["entry_token_budget"] == exact["estimated_tokens"] - 1
        assert exact["error_code"] == "budget-exceeded" and exact["previous_retained"] is True
        old = bytes(cases["reactivated-rule"]["old_entry"])
        new = bytes(cases["reactivated-rule"]["entry"])
        revised = bytes(cases["source-revision"]["entry"])
        assert old == bytes(baseline["entry"]) and b"RULE_" not in old and b"RULE_" in new
        assert b"REVISED_RULE_" in revised and cases["source-revision"]["stale_error"] == "source-changed"
        large = cases["oversized-replacement"]
        assert large["entry_bytes"] == len(large["entry"]) and large["capacity"] == 32768
        assert large["hit"] is False and large["repeat_hit"] is False
        assert large["previous_retained"] is True and large["same_old_arc"] is True
        assert cases["capacity"] == {"one_byte_returns_uncached": True, "zero_capacity_error": "resource-limit"}
        assert cases["clear-preserves-callers"] == {"hit_after_clear": False, "caller_entries_unchanged": True}
    assert sha(war.read_bytes()) == binary
    publish_observation(out / "summary.json", {"kind": "actual-budget-cache-sdk-consumer", "observed_cases": len(cases),
        "sdk_calls": len(calls), "calls": calls, "transcript_sha256": sha(raw),
        "wire_sha256": sha((out / "sdk-wire.jsonl").read_bytes()), "binary_sha256": binary,
        "qualification_established": False, "limitations": ["Accounting is UTF8 bytes/4 estimate, not tokenizer or spend",
        "Cache bound covers declared retained payload, not allocator overhead or caller-held Arc memory",
        "External test instrumentation and SDK decoding do not establish trusted execution or human acceptance"]})
    print("PASS 31 observed cache/accounting cases; 10 SDK calls", flush=True)


if __name__ == "__main__":
    main()
