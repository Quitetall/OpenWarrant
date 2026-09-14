#!/usr/bin/env bash
# A performer that does no work and says so honestly: it answers the Dispatch on
# stdin with a §51.2-legal Stage Submission asking to be verified. It exists so
# `war perform`'s own path is exercised without a model in the loop.
#
# `python3 -c`, not a heredoc: a heredoc would take stdin and the Dispatch would
# never arrive.
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
    "claims": [{"id": "C-001", "statement": "The fixture performer ran and produced no artifact; this claim is the only thing it asserts."}],
    "artifact_refs": [],
    "blockers": [],
    "requested_next_action": "verify",
}, indent=2))
'
