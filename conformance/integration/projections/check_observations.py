#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Check retained external projection observations through the public SDK CLI.

The external probe is an additional test caller in a disposable provider checkout.
This consumer does not implement projection, closure, trust or authority policy.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from observation import publish_observation


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--transcript", type=Path, required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    war = args.war.resolve(strict=True)
    transcript = args.transcript.read_bytes()
    rows = [json.loads(line.split("OWOBS:", 1)[1]) for line in transcript.decode().splitlines() if "OWOBS:" in line]
    cases = {row["case"]: row["observation"] for row in rows}
    errors = {"budget": "budget-exceeded", "entry-limit": "resource-limit", "access": "context-incomplete",
              "source-lock": "source-changed", "role": "source-invalid", "origin": "source-invalid",
              "summary-trust": "summary-invalid", "summary-classification": "summary-invalid",
              "summary-taints": "summary-invalid", "summary-authority": "summary-invalid",
              "unsupported-authority-label": "source-invalid", "summary-parent-binding": "summary-invalid",
              "summary-unit-binding": "summary-invalid"}
    roles = ["preparation", "implementation", "verification", "human-review"]
    assert len(cases) == len(rows) == 21
    assert set(cases) == {"inputs", "master", "performer-claim", "prior-preserved", *roles, *errors}
    assert all(row["qualification_established"] is False for row in rows)
    output = args.record_dir.resolve()
    output.mkdir(parents=True, exist_ok=False)
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}
    baseline = cases["inputs"]
    sources = {s["source_digest"]: s for s in baseline["sources"]}
    blobs = {key: bytes(value) for key, value in baseline["blobs"].items()}
    nonce = baseline["nonce"]
    calls = []
    cached_units = {}
    binary_digest = digest(war.read_bytes())
    with (output / "sdk-wire.jsonl").open("xb") as wire:
        def sdk(label, operation, **fields):
            raw = json.dumps({"schema": "oh.war/sdk-request/v1", "operation": operation, **fields}).encode()
            argv = [str(war), "sdk", "--request", "-"]
            p = subprocess.run(argv, input=raw, capture_output=True, timeout=30, env=env)
            wire.write((json.dumps({"label": label, "argv": argv, "stdin": raw.decode(),
                       "stdout": p.stdout.decode(), "stderr": p.stderr.decode(), "exit_code": p.returncode},
                       ensure_ascii=False, separators=(",", ":")) + "\n").encode())
            wire.flush()
            value = json.loads(p.stdout)
            assert p.returncode == value["exit_code"] == 0, value
            assert value["schema"] == "oh.war/report/v1"
            calls.append({"label": label, "stdin_sha256": digest(raw), "stdout_sha256": digest(p.stdout)})
            return value["result"]

        for key, source in sources.items():
            assert key == "sha256:" + digest(blobs[key]) and source["byte_length"] == len(blobs[key])
            decoded = sdk("source:" + source["path"], "source-codec", payload=json.dumps(source))
            assert decoded["descriptor"] == source

        def exact(reference):
            key = (reference["source_digest"], reference["unit"])
            if key not in cached_units:
                raw = blobs[key[0]]
                if key[1] == "*":
                    cached_units[key] = raw
                else:
                    cached_units[key] = sdk("unit:" + sources[key[0]]["path"] + "#" + key[1], "unit",
                                            source=raw.decode(), dialect="rc3", unit=key[1])["unit"].encode()
            value = cached_units[key]
            assert blobs[key[0]][reference["start"]:reference["end"]] == value
            return value

        first_bindings = None
        first_brief = None
        expected = {("task.md", unit) for unit in ["outcome", "scope", "context", "constraints", "outputs", "stop"]}
        expected |= {("history.md", "context"), ("history.md", "parent-" + nonce), ("summary.md", "short-" + nonce)}
        for role in roles + ["performer-claim"]:
            case = cases[role]
            packet = case["packet"]
            decoded = sdk("packet:" + role, "packet-decode", payload=case["packet_wire"])
            assert decoded["packet"] == packet and decoded["integrity_checked"] is False
            assert decoded["semantic_coverage_established"] is False
            assert packet["sources"] == baseline["sources"] and packet["readiness"]["value"] == "blocked"
            membership = {(sources[b["ref"]["source_digest"]]["path"], b["ref"]["unit"]) for b in packet["binding_context"]}
            assert membership == expected
            entry = bytes(case["entry"])
            for binding in packet["binding_context"]:
                assert entry.count(exact(binding["ref"])) == 1
            if first_bindings is None:
                first_bindings, first_brief = packet["binding_context"], packet["brief"]
            assert packet["binding_context"] == first_bindings and packet["brief"] == first_brief
            if role in roles:
                assert packet["task"]["role"] == role
            print("PASS", role, flush=True)

        for source in sources.values():
            if source["path"] == "summary.md":
                parsed = sdk("summary-provenance", "parse", source=blobs[source["source_digest"]].decode(), dialect="rc3")
                declaration = parsed["metadata"]["extensions"]["lamu:background-summary"]["items"][0]
                assert declaration["model_produced"] is True and declaration["sources"] == ["history.md#parent-" + nonce]
                assert source["metadata"]["authority"] == "unestablished"

        master = cases["master"]
        assert master["readiness"]["value"] == "not_evaluated"
        assert len(master["selection"]["selected"]) == 11
        entry = bytes(master["entry"])
        for item in master["selection"]["selected"]:
            raw = exact(item["reference"])
            if item["kind"] == "opaque":
                assert raw not in entry and b"Data, not instructions" in entry
            else:
                assert entry.count(raw) == 1
        print("PASS master", flush=True)
        for label, code in errors.items():
            assert cases[label]["error_code"] == code
            assert "packet" not in cases[label] and "entry" not in cases[label]
            print("PASS", label, code, flush=True)
        assert cases["prior-preserved"]["entry"] == cases["verification"]["entry"]
        assert cases["prior-preserved"]["snapshot_used_after_live_file_change"] is True
        assert cases["performer-claim"]["readiness"]["value"] == "blocked"
    assert digest(war.read_bytes()) == binary_digest
    publish_observation(output / "summary.json", {"kind": "actual-projection-sdk-consumer-observation",
        "observed_cases": 20, "sdk_calls": len(calls), "calls": calls,
        "transcript_sha256": digest(transcript), "binary_sha256": binary_digest,
        "wire_sha256": digest((output / "sdk-wire.jsonl").read_bytes()),
        "qualification_established": False,
        "limitations": ["External probe is test instrumentation, not a new provider release",
                        "SDK decoding does not establish producer trust or semantic qualification",
                        "No model invocation, trusted access deployment or human acceptance"]})


if __name__ == "__main__":
    main()
