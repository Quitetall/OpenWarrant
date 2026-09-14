# shellcheck shell=bash
# `war pins` / `war next` — the agent-facing questions "what may I not edit?"
# and "whose act is next?". Positive plants: the answers must SAY the right
# thing, not merely exit zero.

# A resolved Warrant's pinned file appears, with its state.
plant_cmd "pins lists a resolved warrant's deliverable" "oh.war/pins/v1" "crates/openwarrant-cli/src/check.rs" 0 \
    "true" \
    --json pins --resolved-only

# Every signing command in `next` belongs to a human. The battery's grep is a
# substring check, so the assertion is on the shape the JSON always has for a
# human act; the unit test asserts the full invariant over the table.
plant_cmd "next hands signatures to humans" "oh.war/next/v1" "\"actor\": \"human\"" 0 \
    "true" \
    --json next

# And nothing an agent is handed is a signature: the string "war sign" must
# never share an action object with actor agent. Checked by a python one-liner
# because the invariant spans two fields.
if ./target/debug/war --json next 2>/dev/null | python3 -c '
import sys, json
v = json.load(sys.stdin)["result"]
bad = [a for a in v["actions"] if a["actor"] == "agent" and a["command"].startswith("war sign")]
sys.exit(1 if bad else 0)'; then
    printf 'ok    %-34s no agent action is a signature\n' "next never hands an agent a signature"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s an agent action is a signature\n' "next never hands an agent a signature"
    FAILED=$((FAILED + 1))
fi
