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
# The corpus answer is read once, timed (t-280c, below) and checked here.
NX_S=$(date +%s%N)
NX_CORPUS=$("$WAR" --json next 2>/dev/null)
NX_RC=$?
NX_MS=$(( ($(date +%s%N) - NX_S) / 1000000 ))
if "$WAR" --root "$PLANT_ROOT" --json next 2>/dev/null | nx_no_agent_signs need \
    && printf '%s' "$NX_CORPUS" | nx_no_agent_signs any; then
    printf 'ok    %-34s no agent action is a signature\n' "next never hands an agent a signature"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s an agent action is a signature, or the scratch handed out none\n' "next never hands an agent a signature"
    FAILED=$((FAILED + 1))
fi

# t-280c: `war next` on this corpus answers in bounded time. It took ~7 s
# (debug build) when the dry run read the whole signing queue a second time
# and every Warrant load re-parsed the SAS revisions; ~3 s after. The bound
# is about 2x that, generous for a loaded machine, and below the old time so
# the regression is caught. A fast answer is only a pass if it is also a
# whole one: it exited 0, is an oh.war/next/v1 report, and hands no agent a
# signature (the refusal above, over the same bytes).
NX_BOUND_MS=6000
if [[ $NX_RC -eq 0 && $NX_MS -lt $NX_BOUND_MS ]] \
    && grep -q '"oh.war/next/v1"' <<<"$NX_CORPUS" \
    && printf '%s' "$NX_CORPUS" | nx_no_agent_signs any; then
    printf 'ok    %-34s %s ms (< %s ms), no agent action is a signature\n' "next answers this corpus in time" "$NX_MS" "$NX_BOUND_MS"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s ms (bound %s ms), exit %s, or an agent action is a signature\n' "next answers this corpus in time" "$NX_MS" "$NX_BOUND_MS" "$NX_RC"
    FAILED=$((FAILED + 1))
fi
unset NX_S NX_CORPUS NX_RC NX_MS NX_BOUND_MS
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
