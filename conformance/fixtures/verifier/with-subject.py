# SPDX-License-Identifier: Apache-2.0
"""Bind a synthetic fixture verdict to the request the fixture actually read.

This is test setup, not independent review or authority. The caller captures
its request before simulating review. Never reads newer repository state.
"""
import json
from pathlib import Path
import sys
import tomllib

request = json.load(open(sys.argv[1]))
assert request["exit_code"] == 0, "fixture request must have succeeded"
index = request["result"]
compact = index.get("schema") == "oh.war/review-context/v3"
if compact:
    # Fixture setup only: read the archive captured by the caller, never a
    # newly assembled request. The explicit root also works with war --root.
    root = Path(sys.argv[3])
    archive = json.loads((root / index["subject_archive"]["path"]).read_text())
    subject = archive["request"]["reviewed_subject"]
else:
    subject = index["reviewed_subject"]
body = open(sys.argv[2]).read()
assert "reviewed_subject" not in tomllib.loads(body), "do not overwrite a review"
head, separator, tail = body.partition("[[verifications]]")
assert separator, "fixture response must contain verdicts"
protocol = "oh.war/verification-response/v3" if compact else "oh.war/verification-response/v2"
print(head.replace("oh.war/verification-response/v1", protocol))
for packet in index["packets"]:
    print("[[reviewed_packets]]")
    print("path = " + json.dumps(packet["path"]))
    print("digest = " + json.dumps(packet["digest"]))
if compact:
    for view in index["views"]:
        print("[[reviewed_views]]")
        print("path = " + json.dumps(view["path"]))
        print("sha256 = " + json.dumps(view["sha256"]))
print("[reviewed_subject]")
print("contract_digest = " + json.dumps(subject["contract_digest"]))
for field in ["artifacts", "context_sources", "gate_definitions", "fixtures", "gate_evidence", "gate_inputs", "gate_links"]:
    print("[reviewed_subject." + field + "]")
    for key, value in sorted(subject.get(field, {}).items()):
        print(json.dumps(key) + " = " + json.dumps(value))
print(separator + tail)
