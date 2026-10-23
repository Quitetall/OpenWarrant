#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe the public SDK author/edit boundary against unchanged author fixtures."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
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
        raise ValueError("observations must be outside the protected fixture directory")
    output.mkdir(parents=True, exist_ok=False)
    wire = (output / "wire.jsonl").open("x", encoding="utf-8")
    calls = []
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}

    def read(name):
        path = (root / name).resolve(strict=True)
        if not path.is_relative_to(root):
            raise ValueError("fixture path escapes the fixture directory")
        return path.read_bytes()

    def call(label, request, extra=()):
        payload = json.dumps(request, ensure_ascii=False, separators=(",", ":")).encode()
        result = subprocess.run([str(binary), "sdk", "--request", "-", *extra],
                                input=payload, capture_output=True, env=env, timeout=30)
        record = {"label": label, "argv": [str(binary), "sdk", "--request", "-", *extra],
                  "stdin": payload.decode(), "stdout": result.stdout.decode(),
                  "stderr": result.stderr.decode(), "exit_code": result.returncode}
        wire.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n")
        wire.flush()
        response = json.loads(result.stdout)
        assert response["schema"] == "oh.war/report/v1" and response["command"] == "sdk"
        assert response["exit_code"] == result.returncode
        calls.append({"label": label, "request": json.loads(payload), "response": response,
                      "exit_code": result.returncode, "stderr": record["stderr"],
                      "stdin_sha256": digest(payload), "stdout_sha256": digest(result.stdout)})
        return response

    def request(operation, **fields):
        return {"schema": "oh.war/sdk-request/v1", "operation": operation, **fields}

    suite = json.loads(read("cases.json"))
    assert suite["schema"] == "oh.war/sdk-author-cases/v1" and suite["cases"]
    ids = set()
    completed = []
    case_files = []
    for case in suite["cases"]:
        assert case["id"] not in ids
        ids.add(case["id"])
        assert (case["expected"] is None) != (case["error"] is None)
        raw = read(case["input"])
        spec = json.loads(raw)
        assert not set(spec) - {"metadata", "units", "crlf", "wrapped", "edits"}
        begin = len(calls)
        authored = call(case["id"] + ":author", request("author", metadata=spec["metadata"],
                        units=spec["units"], crlf=spec.get("crlf", False), wrap=spec.get("wrapped", False)))
        result = authored
        if authored["exit_code"] == 0:
            before = authored["result"]["source"]
            assert authored["result"]["saved"] is False and authored["result"]["qualified"] is False
            parsed = call(case["id"] + ":parse-before", request("parse", source=before, dialect="rc3"))
            assert parsed["exit_code"] == 0
            edits = [{"kind": e["type"], **{k: v for k, v in e.items() if k != "type"}}
                     for e in spec.get("edits", [])]
            result = call(case["id"] + ":edit", request("edit", source=before, dialect="rc3", edits=edits))
            if result["exit_code"] == 0:
                after = result["result"]["source"]
                assert case["expected"] is not None and after.encode() == read(case["expected"])
                if not edits:
                    assert after == before
                validated = call(case["id"] + ":validate-after", request("validate", source=after, dialect="rc3"))
                assert validated["exit_code"] == 0 and validated["result"]["validity"] == "Valid"
                assert validated["result"]["readiness"] == "not-evaluated"
                assert validated["result"]["qualification_established"] is False
                changed_units = {e["id"] for e in edits if e["kind"] == "unit"}
                title_changed = any(e["kind"] == "metadata" and e["key"] == "title" for e in edits)
                for unit in parsed["result"]["units"]:
                    old = call(case["id"] + ":unit-before:" + unit["id"], request("unit", source=before, dialect="rc3", unit=unit["id"]))
                    new = call(case["id"] + ":unit-after:" + unit["id"], request("unit", source=after, dialect="rc3", unit=unit["id"]))
                    assert old["exit_code"] == new["exit_code"] == 0
                    if unit["id"] not in changed_units and not title_changed:
                        assert old["result"]["unit"] == new["result"]["unit"]
        if result["exit_code"]:
            assert case["error"] is not None and result["result"] is None
            assert any(d.get("code") == case["error"] for d in result["diagnostics"])
        else:
            assert case["error"] is None
        record = {"kind": "actual-sdk-author-observation", "case": case["id"],
                  "fixture": case["input"], "fixture_sha256": digest(raw), "calls": calls[begin:],
                  "expectation": {"file": case["expected"], "refusal": case["error"]},
                  "checks_passed": True, "qualification_established": False}
        encoded_record = (json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
        name = "case-" + digest(encoded_record) + ".json"
        case_files.append({"case": case["id"], "path": name})
        (output / name).write_bytes(encoded_record)
        completed.append(case["id"])
        print("PASS", case["id"], flush=True)
    # Exercise preservation across multiple units through the public seam.
    # Expected text comes from the supplied literals, not a second author call.
    begin = len(calls)
    multi = copy.deepcopy(json.loads(read("T54-minimum.json")))
    original_body = multi["units"][0]["text"].replace("\n", "\r\n")
    notes = "## Notes\r\n\r\nUntouched é and literal $(shell).\r\n"
    multi["units"][0]["text"] = original_body
    multi["units"].append({"id": "notes", "kind": "background", "text": notes})
    authored = call("multi-unit:author", request("author", metadata=multi["metadata"],
                    units=multi["units"], crlf=True, wrap=True))
    assert authored["exit_code"] == 0
    original = authored["result"]["source"]
    replacement = "# Author é\r\n\r\nChanged only the binding unit.\r\n"
    edited = call("multi-unit:edit", request("edit", source=original, dialect="rc3",
                  edits=[{"kind": "unit", "id": "context", "text": replacement}]))
    assert edited["exit_code"] == 0
    changed = edited["result"]["source"]
    assert changed == original.replace(original_body, replacement, 1)
    observed_notes = call("multi-unit:untouched", request("unit", source=changed,
                          dialect="rc3", unit="notes"))
    assert observed_notes["exit_code"] == 0 and observed_notes["result"]["unit"] == notes
    revised = call("multi-unit:revision", request("edit", source=changed, dialect="rc3",
                   edits=[{"kind": "metadata", "key": "revision", "value": 2}]))
    assert revised["exit_code"] == 0
    assert revised["result"]["source"].split("<!-- ow:metadata -->", 1)[0] == changed.split("<!-- ow:metadata -->", 1)[0]
    parsed_revision = call("multi-unit:parse-revision", request("parse",
                           source=revised["result"]["source"], dialect="rc3"))
    assert parsed_revision["exit_code"] == 0
    assert parsed_revision["result"]["metadata"]["revision"] == 2
    assert parsed_revision["result"]["metadata"]["id"] == "example:author"
    renamed = call("multi-unit:identity", request("edit", source=revised["result"]["source"],
                   dialect="rc3", edits=[{"kind": "metadata", "key": "id", "value": "example:author-revised"}]))
    assert renamed["exit_code"] == 0
    parsed_identity = call("multi-unit:parse-identity", request("parse",
                           source=renamed["result"]["source"], dialect="rc3"))
    assert parsed_identity["exit_code"] == 0
    assert parsed_identity["result"]["metadata"]["id"] == "example:author-revised"
    assert parsed_identity["result"]["metadata"]["revision"] == 2
    assert renamed["result"]["source"].split("<!-- ow:metadata -->", 1)[0] == changed.split("<!-- ow:metadata -->", 1)[0]
    missing = call("multi-unit:missing", request("edit", source=original, dialect="rc3",
                   edits=[{"kind": "unit", "id": "absent", "text": "No partial output.\n"}]))
    assert missing["exit_code"] != 0 and missing["result"] is None
    assert any(d.get("code") == "unit-missing" for d in missing["diagnostics"])
    (output / "multi-unit-controls.json").write_text(json.dumps({
        "kind": "actual-sdk-multi-unit-controls", "calls": calls[begin:],
        "untouched_unit_exact": True, "untouched_metadata_exact": True,
        "revision_keeps_unit_bytes": True, "identity_change_is_explicit": True,
        "missing_unit_has_no_partial_output": True,
        "qualification_established": False}, ensure_ascii=False, separators=(",", ":")) + "\n")
    # A new, unlisted payload must change the document. No fixture filename is
    # sent to the SDK; it receives only the actual metadata and unit values.
    spec = copy.deepcopy(json.loads(read("T54-minimum.json")))
    marker = "Unlisted author payload " + uuid.uuid4().hex
    for pair in spec["metadata"]:
        if pair[0] == "title": pair[1] = marker
    spec["units"][0]["text"] = "# " + marker + "\n\nNew literal body.\n"
    unseen = call("unseen-content", request("author", metadata=spec["metadata"], units=spec["units"]))
    assert unseen["exit_code"] == 0 and marker in unseen["result"]["source"]
    assert "New literal body." in unseen["result"]["source"]
    with tempfile.TemporaryDirectory(prefix="ow-author-output-") as tmp:
        target = Path(tmp) / "result.json"
        saved = call("new-output", request("author", metadata=spec["metadata"], units=spec["units"]), ("--output", str(target)))
        assert saved["exit_code"] == 0 and json.loads(target.read_bytes()) == saved
        previous = target.read_bytes()
        refused = call("existing-output", request("author", metadata=spec["metadata"], units=spec["units"]), ("--output", str(target)))
        assert refused["exit_code"] != 0 and target.read_bytes() == previous
    wire.close()
    (output / "controls.json").write_text(json.dumps({"kind": "actual-sdk-author-controls", "calls": calls[-3:], "existing_output_unchanged": True, "checks_passed": True, "qualification_established": False}, ensure_ascii=False, separators=(",", ":")) + "\n")
    (output / "summary.json").write_text(json.dumps({"cases": completed, "case_files": case_files, "sdk_calls": len(calls), "binary_sha256": digest(binary.read_bytes()), "wire_sha256": digest((output / "wire.jsonl").read_bytes()), "qualification_established": False}, indent=2) + "\n")
    print(f"PASS {len(completed)} author cases; unseen content and protected prior output observed", flush=True)


if __name__ == "__main__":
    main()
