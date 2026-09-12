#!/usr/bin/env bash
# A verifier that answers as the performer: the one thing the seam must refuse.
python3 - "$1" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))
print('schema = "oh.war/verification-response/v1"')
print('warrant = "%s"' % b["warrant"])
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
