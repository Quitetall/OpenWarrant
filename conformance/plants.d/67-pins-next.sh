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
#
# Over a scratch program with acts pending (lib.sh, `scratch_queue`): this
# corpus's queue is empty whenever the owner has signed everything, and an
# empty queue hands nobody a signature, so the claim would have nothing to
# be about.
PLANT_ROOT=$(scratch_queue NX)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
plant_cmd "next hands signatures to humans" "oh.war/next/v1" "\"actor\": \"human\"" 0 \
    "true" \
    --json next

# And nothing an agent is handed is a signature: the string "war sign" must
# never share an action object with actor agent. Checked by a python one-liner
# because the invariant spans two fields — over the scratch, where signatures
# are handed out (at least one, or the check proved nothing), and over this
# corpus, whatever its queue holds today.
nx_no_agent_signs() { python3 -c '
import sys, json
v = json.load(sys.stdin)["result"]
signs = [a for a in v["actions"] if a["command"].startswith("war sign")]
bad = [a for a in signs if a["actor"] == "agent"]
sys.exit(1 if bad or (sys.argv[1] == "need" and not signs) else 0)' "$1"; }
if "$WAR" --root "$PLANT_ROOT" --json next 2>/dev/null | nx_no_agent_signs need \
    && "$WAR" --json next 2>/dev/null | nx_no_agent_signs any; then
    printf 'ok    %-34s no agent action is a signature\n' "next never hands an agent a signature"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s an agent action is a signature, or the scratch handed out none\n' "next never hands an agent a signature"
    FAILED=$((FAILED + 1))
fi
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
