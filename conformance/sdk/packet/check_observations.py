#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe F7 codec/integrity and consume real external package observations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "integration"))
from observation import publish_observation


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--transcript", type=Path, required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    war = args.war.resolve(strict=True)
    fixtures = args.fixtures.resolve(strict=True)
    output = args.record_dir.resolve()
    if output == fixtures or output.is_relative_to(fixtures):
        raise ValueError("observations must be outside protected fixtures")
    output.mkdir(parents=True, exist_ok=False)
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}
    binary_sha = digest(war.read_bytes())
    fixture_sha = {}
    calls = []
    with (output / "sdk-wire.jsonl").open("xb") as wire:
        def call(label, operation, error=None, **fields):
            raw = json.dumps({"schema": "oh.war/sdk-request/v1", "operation": operation, **fields}).encode()
            argv = [str(war), "sdk", "--request", "-"]
            p = subprocess.run(argv, input=raw, capture_output=True, timeout=30, env=env)
            wire.write((json.dumps({"label": label, "argv": argv, "stdin": raw.decode(),
                       "stdout": p.stdout.decode(), "stderr": p.stderr.decode(), "exit_code": p.returncode},
                       ensure_ascii=False, separators=(",", ":")) + "\n").encode())
            wire.flush()
            value = json.loads(p.stdout)
            assert value["schema"] == "oh.war/report/v1" and value["exit_code"] == p.returncode
            if error:
                assert p.returncode != 0 and value["result"] is None
                assert any(d.get("code") == error for d in value["diagnostics"]), value
            else:
                assert p.returncode == 0, value
            calls.append({"label": label, "stdin_sha256": digest(raw), "stdout_sha256": digest(p.stdout)})
            return value["result"]

        raw = (fixtures / "cases.json").read_bytes()
        fixture_sha["cases.json"] = digest(raw)
        suite = json.loads(raw)
        assert suite["schema"] == "oh.war/sdk-packet-cases/v1"
        ids = set()
        for case in suite["cases"]:
            assert case["id"] not in ids
            ids.add(case["id"])
            path = (fixtures / case["file"]).resolve(strict=True)
            if not path.is_relative_to(fixtures):
                raise ValueError("fixture path escapes root")
            raw = path.read_bytes()
            fixture_sha[case["file"]] = digest(raw)
            result = call(case["id"], "packet-decode", case["error"], payload=raw.decode())
            if case["error"] is None:
                assert result["integrity_checked"] is False and result["semantic_coverage_established"] is False
                encoded = call(case["id"] + ":encode", "packet-encode", payload=raw.decode())
                assert encoded["payload"].encode() == raw
            print("PASS", case["id"], flush=True)

        transcript = args.transcript.read_bytes()
        rows = [json.loads(line.split("OWOBS:", 1)[1]) for line in transcript.decode().splitlines() if "OWOBS:" in line]
        cases = {row["case"]: row["observation"] for row in rows}
        assert len(cases) == len(rows) == 12
        assert all(row["qualification_established"] is False for row in rows)
        roots = []
        for label in ["valid", "resealed-incomplete"]:
            files = cases[label]["files"]
            result = call(label, "package-check", files=[{"path": name, "hex": bytes(raw).hex()} for name, raw in files.items()])
            assert result["integrity_checked"] is True and result["semantic_coverage_established"] is False
            assert result["packet"]["readiness"]["value"] == "blocked"
            roots.append(result["manifest"]["root_digest"])
            if label == "resealed-incomplete":
                assert cases[label]["provider_error"] == "semantic-mismatch"
                assert not result["packet"]["brief"]["context"]
            print("PASS", label, "integrity only", flush=True)
        assert roots[0] != roots[1]
        for label in ["tamper-ENTRY.md", "tamper-packet.json"]:
            call(label, "package-check", "digest-mismatch",
                 files=[{"path": name, "hex": bytes(raw).hex()} for name, raw in cases[label]["files"].items()])
        for label in ["publication-collision", "publication-traversal", "publication-symlink", "linked-entry", "extra-path", "special-entry"]:
            assert cases[label]["error_code"]
        assert cases["publication-collision"]["original_preserved"] is True
        assert cases["publication-collision"]["no_extra_paths"] is True
        assert cases["entry-limit"]["error_code"] == "resource-limit"
        assert cases["offline-after-source-removal"]["provider_selection_coverage"] is True
        assert cases["offline-after-source-removal"]["readiness"]["value"] == "blocked"
    assert digest(war.read_bytes()) == binary_sha
    publish_observation(output / "summary.json", {"kind": "actual-package-sdk-observations",
        "literal_cases": sorted(ids), "external_cases": 12, "sdk_calls": len(calls), "calls": calls,
        "fixture_sha256": fixture_sha, "binary_sha256": binary_sha,
        "transcript_sha256": digest(transcript), "wire_sha256": digest((output / "sdk-wire.jsonl").read_bytes()),
        "qualification_established": False,
        "limitations": ["SDK integrity is not context completeness or human assurance",
                        "External probe is additional test instrumentation, not a provider release",
                        "Linux filesystem observation only; parent capability is caller-controlled"]})


if __name__ == "__main__":
    main()
