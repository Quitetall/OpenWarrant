#!/usr/bin/env bash
# A verifier that answers as the performer: the one thing the seam must refuse.
python3 - "$1" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))
print('schema = "oh.war/verification-response/v2"')
print('warrant = "%s"' % b["warrant"])
# Echo only the snapshot actually carried to this fixture. Never recapture
# repository state after the simulated review.
import os
for packet in json.loads(os.environ.get("OPENWARRANT_REVIEWED_PACKETS", "[]")):
    print("[[reviewed_packets]]")
    print("path = " + json.dumps(packet["path"]))
    print("digest = " + json.dumps(packet["digest"]))
subject = b["request"].get("reviewed_subject")
if subject is not None:
    print("[reviewed_subject]")
    print("contract_digest = " + json.dumps(subject["contract_digest"]))
    for field in ["artifacts", "context_sources", "gate_definitions", "fixtures", "gate_evidence", "gate_inputs", "gate_links"]:
        print("[reviewed_subject." + field + "]")
        for key, value in sorted(subject.get(field, {}).items()):
            print(json.dumps(key) + " = " + json.dumps(value))
for o in b["request"]["obligations"]:
    print("")
    print("[[verifications]]")
    print('obligation = "%s"' % o["id"])
    print('disposition = "established"')
    print('evidence = "I checked my own work"')
    print('performer = "%s"' % b["request"]["performer"])
    print("[verifications.verifier]")
    print('actor = "%s"' % b["request"]["performer"])
    print('kind = "agent"')
    print("[verifications.verifier.independence]")
    print('performer_transcript_blind = false')
    print('performer_rationale_blind = false')
    print('separate_writable_workspace = false')
    print('cannot_modify_subject_artifacts = false')
    print('cannot_modify_gate_definition = false')
    print('cannot_modify_gate_fixtures = false')
    print('separate_context_compilation = false')
    print('distinct_model_required = false')
    print('distinct_human_required = false')
PY
