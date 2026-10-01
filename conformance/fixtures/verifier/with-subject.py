# SPDX-License-Identifier: Apache-2.0
"""Bind a synthetic fixture verdict to the request the fixture actually read.

This is test setup, not independent review or authority. The caller captures
its request before simulating review. Never reads newer repository state.
"""
import json
import sys
import tomllib

request = json.load(open(sys.argv[1]))
assert request["exit_code"] == 0, "fixture request must have succeeded"
subject = request["result"]["reviewed_subject"]
body = open(sys.argv[2]).read()
assert "reviewed_subject" not in tomllib.loads(body), "do not overwrite a review"
head, separator, tail = body.partition("[[verifications]]")
assert separator, "fixture response must contain verdicts"
print(head)
print("[reviewed_subject]")
print("contract_digest = " + json.dumps(subject["contract_digest"]))
for field in ["artifacts", "gate_definitions", "fixtures", "gate_evidence", "gate_inputs", "gate_links"]:
    print("[reviewed_subject." + field + "]")
    for key, value in sorted(subject.get(field, {}).items()):
        print(json.dumps(key) + " = " + json.dumps(value))
print(separator + tail)
