#!/usr/bin/env bash
# A performer that asks to be resolved. §51.2 says it may ask to continue, be
# verified, block, amend or cancel — never this. The refusal must arrive before
# anything is written.
set -euo pipefail
exec python3 -c '
import json, sys
d = json.load(sys.stdin)
print(json.dumps({
    "schema": "oh.war/stage-submission/v1",
    "dispatch_id": d["dispatch_id"],
    "attempt_id": d["attempt_id"],
    "contract_digest": d["contract_digest"],
    "stage_id": d["stage_id"],
    "claims": [{"id": "C-001", "statement": "I did the work and it is done."}],
    "artifact_refs": [],
    "blockers": [],
    "requested_next_action": "resolve",
}, indent=2))
'
