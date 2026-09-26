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
# nx_cpu_ms <war args...>: run `war <args>` on this corpus; print "<cpu ms>
# <wall ms> <exit>", CPU being user + system of war and the git it runs. Its
# stdout goes to $NX_OUT.
NX_TMP=$(mktemp -d)
NX_OUT="$NX_TMP/out"
nx_cpu_ms() {
    local rc saved_tf="${TIMEFORMAT-}" had_tf="${TIMEFORMAT+set}"
    TIMEFORMAT='%3R %3U %3S'
    { time "$WAR" "$@" > "$NX_OUT" 2>/dev/null; } 2> "$NX_TMP/time"
    rc=$?
    if [[ -n "$had_tf" ]]; then TIMEFORMAT="$saved_tf"; else unset TIMEFORMAT; fi
    awk -v rc="$rc" '{ printf "%d %d %d\n", ($2 + $3) * 1000, $1 * 1000, rc }' < <(tail -1 "$NX_TMP/time")
}
read -r NX_MS NX_WALL_MS NX_RC < <(nx_cpu_ms --json next)
NX_CORPUS=$(cat "$NX_OUT")
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
# What is timed, and against what (t-d052). A wall-clock bound failed with
# nothing regressed when three batteries ran at once (6.2-7.7 s against 6000
# ms): wall time counts every moment `war` waited for a core. CPU time does
# not, but on its own it still moved 1.45x under load on this machine (a
# hybrid CPU: a process that lands on an efficiency core spends more CPU on
# the same work). So the bound is RELATIVE: `war next`'s CPU over `war
# status`'s, the same corpus loaded by a command without next's work, timed
# in the same run, interleaved, the lowest of three each. Load moves both;
# the ratio stays. Measured on one machine: before t-280c next/status =
# 11.35/2.63 = 4.3, after 4.84/2.06 = 2.35 (corpus 933448d2, load average
# 42, single samples); at this corpus with t-eca6's memo, 2.5-2.6 (min of
# three, three runs, load average 13). The t-280c fix shifted the ratio by
# 1.85x, so it would read ~4.7 here; NX_RATIO_BOUND sits between, ~1.35x
# from each. A regression that slows both alike (the
# t-eca6 walk per receipt: next 17 s CPU, status 68 s) is what the absolute
# backstop is for: above any load seen (5.5 s CPU three-wide), far below that.
NX_RATIO_BOUND=3.4
NX_ABS_BOUND_MS=15000
NX_ST_MS=
for NX_I in 1 2 3; do
    read -r NX_S_MS _ NX_S_RC < <(nx_cpu_ms --json status)
    if [[ $NX_S_RC -eq 0 ]] && [[ -z $NX_ST_MS || $NX_S_MS -lt $NX_ST_MS ]]; then NX_ST_MS=$NX_S_MS; fi
    [[ $NX_I -eq 3 ]] && break
    read -r NX_S_MS _ NX_S_RC < <(nx_cpu_ms --json next)
    if [[ $NX_S_RC -eq 0 && $NX_S_MS -lt $NX_MS ]]; then NX_MS=$NX_S_MS; fi
done
NX_RATIO=$(awk -v n="$NX_MS" -v s="${NX_ST_MS:-0}" 'BEGIN { if (s > 0) printf "%.2f", n / s; else print "none" }')
if [[ $NX_RC -eq 0 && $NX_RATIO != none && $NX_MS -lt $NX_ABS_BOUND_MS ]] \
    && awk -v r="$NX_RATIO" -v b="$NX_RATIO_BOUND" 'BEGIN { exit !(r < b) }' \
    && grep -q '"oh.war/next/v1"' <<<"$NX_CORPUS" \
    && printf '%s' "$NX_CORPUS" | nx_no_agent_signs any; then
    printf 'ok    %-34s next/status CPU %s (< %s; %s/%s ms; %s ms wall), no agent action is a signature\n' "next answers this corpus in time" "$NX_RATIO" "$NX_RATIO_BOUND" "$NX_MS" "$NX_ST_MS" "$NX_WALL_MS"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s next/status CPU %s (bound %s; %s/%s ms, backstop %s ms), exit %s, or an agent action is a signature\n' "next answers this corpus in time" "$NX_RATIO" "$NX_RATIO_BOUND" "$NX_MS" "${NX_ST_MS:-none}" "$NX_ABS_BOUND_MS" "$NX_RC"
    FAILED=$((FAILED + 1))
fi
rm -rf "$NX_TMP"
unset NX_TMP NX_OUT NX_CORPUS NX_RC NX_MS NX_WALL_MS NX_ST_MS NX_RATIO NX_RATIO_BOUND NX_ABS_BOUND_MS NX_I NX_S_MS NX_S_RC
unset -f nx_cpu_ms
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
if [[ -n $NX_ITEM ]] && ! { NX_NEXT=$("$WAR" --root "$PLANT_ROOT" next 2>/dev/null) && grep -qF "war claim $NX_ITEM" <<<"$NX_NEXT"; }; then
    printf 'ok    %-34s a claimed item is no longer offered\n' "next refuses a claimed ticket item"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s is still offered after its claim\n' "next refuses a claimed ticket item" "${NX_ITEM:-?}"
    FAILED=$((FAILED + 1))
fi
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
