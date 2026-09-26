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
# nx_timed_next: run `war --json next` on this corpus; set NX_CORPUS, NX_RC,
# NX_MS (CPU: user + system, `war` and the git it runs) and NX_WALL_MS.
nx_timed_next() {
    local tmp times saved_tf="${TIMEFORMAT-}" had_tf="${TIMEFORMAT+set}"
    tmp=$(mktemp -d)
    TIMEFORMAT='%3R %3U %3S'
    { time "$WAR" --json next > "$tmp/out" 2>/dev/null; } 2> "$tmp/time"
    NX_RC=$?
    if [[ -n "$had_tf" ]]; then TIMEFORMAT="$saved_tf"; else unset TIMEFORMAT; fi
    NX_CORPUS=$(cat "$tmp/out")
    times=$(tail -1 "$tmp/time")
    NX_WALL_MS=$(awk '{ printf "%d", $1 * 1000 }' <<<"$times")
    NX_MS=$(awk '{ printf "%d", ($2 + $3) * 1000 }' <<<"$times")
    rm -rf "$tmp"
}
nx_timed_next
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
# and every Warrant load re-parsed the SAS revisions; ~3 s after. A fast
# answer is only a pass if it is also a whole one: it exited 0, is an
# oh.war/next/v1 report, and hands no agent a signature (the refusal above,
# over the same bytes).
#
# The bound is on CPU time, not wall time (t-d052). The regression was work
# done twice, and work is CPU; wall time also counts every moment the process
# waited for a core, so three batteries at once on a loaded machine read
# 6.2-7.7 s against the old 6000 ms wall bound with nothing regressed. CPU time
# does not count the wait. Measured on one corpus (933448d2) at load average
# ~42: the old binary 11.3-11.5 s CPU, the fixed one 4.8-4.9 s; idle, the old
# one ~7 s. 6000 ms of CPU sits between them. A sample over the bound is taken
# once more and the lower counts: a regression is over it both times, a
# single descheduled-and-migrated sample is not.
NX_BOUND_MS=6000
if [[ $NX_RC -eq 0 && $NX_MS -ge $NX_BOUND_MS ]]; then
    NX_FIRST_MS=$NX_MS
    nx_timed_next
    [[ $NX_FIRST_MS -lt $NX_MS ]] && NX_MS=$NX_FIRST_MS
fi
if [[ $NX_RC -eq 0 && $NX_MS -lt $NX_BOUND_MS ]] \
    && grep -q '"oh.war/next/v1"' <<<"$NX_CORPUS" \
    && printf '%s' "$NX_CORPUS" | nx_no_agent_signs any; then
    printf 'ok    %-34s %s ms CPU (< %s ms; %s ms wall), no agent action is a signature\n' "next answers this corpus in time" "$NX_MS" "$NX_BOUND_MS" "$NX_WALL_MS"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s ms CPU (bound %s ms; %s ms wall), exit %s, or an agent action is a signature\n' "next answers this corpus in time" "$NX_MS" "$NX_BOUND_MS" "$NX_WALL_MS" "$NX_RC"
    FAILED=$((FAILED + 1))
fi
unset NX_CORPUS NX_RC NX_MS NX_WALL_MS NX_BOUND_MS NX_FIRST_MS
unset -f nx_timed_next
# t-67ed: a ready ticket item is listed before any human act, as a claim.
# The human acts stay listed after it, still judged, and still a human's.
# Refusal: once the item is claimed it is no longer offered.
"$WAR" --root "$PLANT_ROOT" create "Plant ticket" --item "Ready plant item" >/dev/null 2>&1
NX_HUMAN=$("$WAR" --root "$PLANT_ROOT" next 2>/dev/null)
NX_CLAIM_AT=$(grep -n -m1 '^ready .*war claim t-' <<<"$NX_HUMAN" | cut -d: -f1)
NX_SIGN_AT=$(grep -n -m1 '^HUMAN ' <<<"$NX_HUMAN" | cut -d: -f1)
nx_ready_first() { python3 -c '
import sys, json
v = json.load(sys.stdin)["result"]
r = v.get("ready", [])
ok = any(x["text"] == "Ready plant item" for x in r)
ok = ok and all(x["actor"] == "agent" and x["command"].startswith("war claim ") and "war sign" not in x["command"] for x in r)
humans = [a for a in v["actions"] if a["actor"] == "human"]
ok = ok and humans and all(a["command"].startswith("war sign") and a.get("judged") for a in humans)
sys.exit(0 if ok else 1)'; }
if [[ -n $NX_CLAIM_AT && -n $NX_SIGN_AT ]] && (( NX_CLAIM_AT < NX_SIGN_AT )) \
    && "$WAR" --root "$PLANT_ROOT" --json next 2>/dev/null | nx_ready_first; then
    printf 'ok    %-34s the claim before the first HUMAN act\n' "next lists ready tickets first"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s claim@%s human@%s\n' "next lists ready tickets first" "${NX_CLAIM_AT:-none}" "${NX_SIGN_AT:-none}"
    FAILED=$((FAILED + 1))
fi
NX_ITEM=$("$WAR" --root "$PLANT_ROOT" --json ready 2>/dev/null | python3 -c '
import sys, json
r = json.load(sys.stdin)["result"]["ready"]
print(next(x["ticket"] + "/" + x["item"] for x in r if x["text"] == "Ready plant item"))' 2>/dev/null)
"$WAR" --root "$PLANT_ROOT" claim "$NX_ITEM" --as plant-agent >/dev/null 2>&1
if [[ -n $NX_ITEM ]] && ! "$WAR" --root "$PLANT_ROOT" next 2>/dev/null | grep -qF "war claim $NX_ITEM"; then
    printf 'ok    %-34s a claimed item is no longer offered\n' "next refuses a claimed ticket item"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s is still offered after its claim\n' "next refuses a claimed ticket item" "${NX_ITEM:-?}"
    FAILED=$((FAILED + 1))
fi
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
