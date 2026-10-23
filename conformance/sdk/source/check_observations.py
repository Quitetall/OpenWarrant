#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe supplied-source SDK operations; acquire no document sources implicitly."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import uuid


def sha(raw):
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
        raise ValueError("observations must be outside protected source fixtures")
    output.mkdir(parents=True, exist_ok=False)
    calls = []
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}
    wire = (output / "wire.jsonl").open("x", encoding="utf-8")

    def read(name):
        path = (root / name).resolve(strict=True)
        if not path.is_relative_to(root):
            raise ValueError("fixture path escapes its source root")
        return path.read_bytes()

    def request(operation, **fields):
        return {"schema": "oh.war/sdk-request/v1", "operation": operation, **fields}

    def call(label, payload, extra=()):
        raw = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode()
        argv = [str(binary), "sdk", "--request", "-", *extra]
        p = subprocess.run(argv, input=raw, capture_output=True, env=env, timeout=30)
        entry = {"label": label, "argv": argv, "stdin": raw.decode(),
                 "stdout": p.stdout.decode(), "stderr": p.stderr.decode(), "exit_code": p.returncode}
        wire.write(json.dumps(entry, ensure_ascii=False, separators=(",", ":")) + "\n")
        wire.flush()
        response = json.loads(p.stdout)
        assert response["schema"] == "oh.war/report/v1" and response["command"] == "sdk"
        assert response["exit_code"] == p.returncode
        calls.append({"label": label, "request": payload, "response": response,
                      "exit_code": p.returncode, "stdin_sha256": sha(raw), "stdout_sha256": sha(p.stdout)})
        return response

    suite = json.loads(read("cases.json"))
    assert suite["schema"] == "oh.war/sdk-source-cases/v1" and suite["cases"]
    ids = set()
    case_files = []
    for case in suite["cases"]:
        assert case["id"] not in ids
        ids.add(case["id"])
        assert (case["error"] is None) != (case["expected_lock"] is None)
        begin = len(calls)
        descriptors = []
        blobs = {}
        result = None
        for index, source in enumerate(case["sources"]):
            raw_descriptor = read(source["descriptor"])
            result = call(f"{case['id']}:descriptor:{index}",
                          request("source-codec", payload=raw_descriptor.decode()))
            if result["exit_code"]:
                break
            descriptor = result["result"]["descriptor"]
            assert descriptor == json.loads(raw_descriptor)
            descriptors.append(descriptor)
            blob = read(source["blob"])
            digest = descriptor["source_digest"]
            if digest in blobs and blobs[digest] != blob:
                raise ValueError("fixture reuses one digest for conflicting blob bytes")
            blobs[digest] = blob
        files = [{"path": digest, "hex": raw.hex()} for digest, raw in blobs.items()]
        if result is not None and not result["exit_code"]:
            result = call(case["id"] + ":lock", request("source-lock", descriptors=descriptors, files=files))
            if not result["exit_code"]:
                assert result["result"]["source_count"] == len(descriptors)
                assert result["result"]["semantic_coverage_established"] is False
                if case["expected_lock"] is not None:
                    assert result["result"]["canonical"].encode() == read(case["expected_lock"])
                for index, ref in enumerate(case["references"]):
                    result = call(f"{case['id']}:reference:{index}", request("sources-check",
                                  descriptors=descriptors, files=files, reference=ref["reference"]))
                    if result["exit_code"]:
                        break
                    assert bytes.fromhex(result["result"]["selected_hex"]) == read(ref["expected"])
                    assert result["result"]["semantic_coverage_established"] is False
        assert result is not None
        if result["exit_code"]:
            assert case["error"] is not None and result["result"] is None
            assert any(d.get("code") == case["error"] for d in result["diagnostics"])
        else:
            assert case["error"] is None
            for index, descriptor in enumerate(descriptors):
                schema = descriptor.get("document", {}).get("schema")
                dialect = None if schema is None else {
                    "oh.war/document/1.0.0-rc.3": "rc3",
                    "oh.war/document/1.0.0-rc.2": "rc2",
                }[schema]
                derived = call(f"{case['id']}:describe:{index}", request("source-describe",
                               path=descriptor["path"], bytes_hex=blobs[descriptor["source_digest"]].hex(),
                               holder=descriptor["holder"], metadata=descriptor["metadata"], dialect=dialect))
                assert derived["exit_code"] == 0 and derived["result"] == descriptor
        record = {"kind": "actual-sdk-source-observation", "case": case["id"],
                  "fixture_sources": case["sources"], "calls": calls[begin:],
                  "expectation": {"lock": case["expected_lock"], "refusal": case["error"]},
                  "checks_passed": True, "qualification_established": False}
        raw = (json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
        name = "case-" + sha(raw) + ".json"
        (output / name).write_bytes(raw)
        case_files.append({"case": case["id"], "path": name})
        print("PASS", case["id"], flush=True)
    begin = len(calls)
    raw = b"Unlisted opaque source\0" + uuid.uuid4().bytes + b"\xff"
    metadata = {"trust": "untrusted", "classification": "internal", "taints": [],
                "authority": "unestablished", "basis_refs": []}
    described = call("unseen:describe", request("source-describe", path="unlisted/source.bin",
                     bytes_hex=raw.hex(), holder={"kind": "example", "locator": "example:unseen"},
                     metadata=metadata, dialect=None))
    assert described["exit_code"] == 0
    descriptor = described["result"]
    assert descriptor["source_digest"] == "sha256:" + sha(raw) and descriptor["byte_length"] == len(raw)
    check = request("source-lock", descriptors=[descriptor], files=[{"path": descriptor["source_digest"], "hex": raw.hex()}])
    locked = call("unseen:lock", check)
    assert locked["exit_code"] == 0 and json.loads(locked["result"]["canonical"]) == [descriptor]
    with tempfile.TemporaryDirectory(prefix="ow-source-output-", dir=output) as temp:
        target = Path(temp) / "result.json"
        saved = call("output:new", check, ("--output", str(target)))
        assert saved["exit_code"] == 0 and json.loads(target.read_bytes()) == saved
        previous = target.read_bytes()
        refused = call("output:existing", check, ("--output", str(target)))
        assert refused["exit_code"] != 0 and previous == target.read_bytes()
    wire.close()
    (output / "controls.json").write_text(json.dumps({"kind": "actual-sdk-source-controls", "calls": calls[begin:],
        "unseen_bytes_checked": True, "previous_output_preserved": True, "qualification_established": False},
        ensure_ascii=False, separators=(",", ":")) + "\n")
    (output / "summary.json").write_text(json.dumps({"cases": case_files, "sdk_calls": len(calls),
        "binary_sha256": sha(binary.read_bytes()), "wire_sha256": sha((output / "wire.jsonl").read_bytes()),
        "qualification_established": False}, indent=2) + "\n")
    print(f"PASS {len(ids)} source cases and unseen/protected-output controls", flush=True)


if __name__ == "__main__":
    main()
