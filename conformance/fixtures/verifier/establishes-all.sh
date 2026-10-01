#!/usr/bin/env bash
# A fixture verifier: reads the bundle it is handed and establishes every
# obligation as a distinct agent. Exists to exercise the seam, not to judge.
python3 - "$1" <<'PY'
import json, sys
b = json.load(open(sys.argv[1]))
print('schema = "oh.war/verification-response/v1"')
print('warrant = "%s"' % b["warrant"])
# Echo only the snapshot actually carried to this fixture. Never recapture
# repository state after the simulated review.
subject = b["request"].get("reviewed_subject")
if subject is not None:
    print("[reviewed_subject]")
    print("contract_digest = " + json.dumps(subject["contract_digest"]))
    for field in ["artifacts", "gate_definitions", "fixtures", "gate_evidence", "gate_inputs", "gate_links"]:
        print("[reviewed_subject." + field + "]")
        for key, value in sorted(subject.get(field, {}).items()):
            print(json.dumps(key) + " = " + json.dumps(value))
for o in b["request"]["obligations"]:
    print("")
    print("[[verifications]]")
    print('obligation = "%s"' % o["id"])
    print('disposition = "established"')
    print('evidence = "fixture verifier: bundle %s carried the atoms and deliverables this obligation names"' % b["schema"])
    print('performer = "%s"' % b["request"]["performer"])
    print("[verifications.verifier]")
    print('actor = "fixture-verifier"')
    print('kind = "agent"')
    print("[verifications.verifier.independence]")
    print('performer_transcript_blind = true')
    print('performer_rationale_blind = true')
    print('separate_writable_workspace = true')
    print('cannot_modify_subject_artifacts = true')
    print('cannot_modify_gate_definition = true')
    print('cannot_modify_gate_fixtures = true')
    print('separate_context_compilation = true')
    print('distinct_model_required = true')
    print('distinct_human_required = true')
PY
