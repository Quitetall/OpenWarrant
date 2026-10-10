# shellcheck shell=bash
# OW-WAR-0148 M15 — what `war evidence go` does with work that does not land.
#
# One scratch program (GF), six light Warrants of one item each, all run at
# once with `[go] max_attempts = 2` and a 15-second time budget. The
# fixture performer (conformance/fixtures/go/performer.py, no model) is told
# per node and attempt what to do (GO_SCRIPT):
#   K: killed (SIGKILL) on its first attempt;
#   S: sleeps past the time budget on its first attempt;
#   X and Y: both write shared.txt on their first attempt, from one base;
#   C: carries a test (`test -f work/ok.txt`) its first attempt does not
#      satisfy, and its second does;
#   P: killed on every attempt.
#
# Accepted: K, S, C and whichever of X and Y conflicted are each sent back
# once (crashed, timeout, check-failed, conflict), with a note on their
# Warrant saying so, and land on their second attempt: C at `observed`, the
# rest at `claimed`. P is never ticked and holds no claim; after two attempts
# it is set aside for a person, and `war next` lists it with the command that
# puts it back; `war evidence go --retry` does, and `war next` then lists it
# no more.
# Refused: the first X/Y to land wins and the other is not landed over it
# (exactly one conflict); a check that fails ticks nothing (C's first
# attempt); a second `--retry` of a node not set aside is refused by name
# (go.not-blocked).

echo "== war go: crashes, timeouts, conflicts, failed checks (M15) =="
GF_ROOT=$(scratch_corpus GF)
[[ -d "${GF_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
GF_TMP=$(mktemp -d)
GF_WAR="$REPO_ROOT/${WAR#./}"
GF_FIX="$REPO_ROOT/conformance/fixtures/go/performer.py"
gf_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
gf_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
gfw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID GO_LOG="$GF_TMP/go.log" \
    GO_SCRIPT="$GF_TMP/script" GO_SLEEP=0.05 "$GF_WAR" --root "$GF_ROOT" "$@" </dev/null; }
gf_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
gf_new() { gf_field "$(gfw --json create "$@" 2>/dev/null)" 'v["result"]["id"] + "/" + v["result"]["items"][0]["id"]'; }

GF_K=$(gf_new "Killed once" -i k1)
GF_S=$(gf_new "Slow once" -i s1)
GF_X=$(gf_new "Writes shared X" -i x1)
GF_Y=$(gf_new "Writes shared Y" -i y1)
GF_C=$(gf_new "Checked" -i c1)
GF_P=$(gf_new "Always killed" -i p1)
gfw add "${GF_C%/*}" --test 'test -f work/ok.txt' --name ok >/dev/null 2>&1
cat >> "$GF_ROOT/openwarrant.toml" <<EOF

[go]
harness_argv = ["python3", "$GF_FIX"]
max_parallel = 6
max_attempts = 2
node_timeout_secs = 15
EOF
cat > "$GF_TMP/script" <<EOF
$GF_K 1 kill
$GF_S 1 sleep 120
$GF_X 1 write shared.txt X
$GF_Y 1 write shared.txt Y
$GF_C 2 write work/ok.txt yes
$GF_P * kill
EOF
git -C "$GF_ROOT" add -A >/dev/null 2>&1
git -C "$GF_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "six Warrants" >/dev/null 2>&1

GF_OUT=$(gfw --json evidence go 2>/dev/null); GF_RC=$?
GF_SUM=$(python3 - "$GF_OUT" <<'PY'
import json, sys
r = json.loads(sys.argv[1])["result"]
back = {x["node"]: x["outcome"] for x in r["requeued"]}
landed = {x["node"]: (x["attempt"], x["level"]) for x in r["landed"]}
blocked = [x["node"] for x in r["blocked"]]
print(json.dumps({"back": back, "landed": landed, "blocked": blocked, "stop": r["stop"]["reason"]}))
PY
)
gf_get() { gf_field "$GF_SUM" "$1"; }
GF_KB=$(gf_get "v['back'].get('$GF_K')"); GF_KL=$(gf_get "v['landed'].get('$GF_K')")
if [[ $GF_RC -eq 0 && "$GF_KB" == "crashed" && "$GF_KL" == "[2, 'claimed']" ]] \
    && grep -q "attempt 1 of 2 did not land (crashed)" "$GF_ROOT/docs/tickets/${GF_K%/*}/atoms/10-intent.md"; then
    gf_ok "a killed performer is retried" "$GF_K: crashed, noted, open and unclaimed; landed on attempt 2"
else
    gf_fail "a killed performer is retried" "exit $GF_RC; back '$GF_KB', landed '$GF_KL'"
fi
GF_SB=$(gf_get "v['back'].get('$GF_S')"); GF_SL=$(gf_get "v['landed'].get('$GF_S')")
if [[ "$GF_SB" == "timeout" && "$GF_SL" == "[2, 'claimed']" ]]; then
    gf_ok "a timeout is stopped and retried" "$GF_S: killed at its 15s budget, landed on attempt 2"
else
    gf_fail "a timeout is stopped and retried" "back '$GF_SB', landed '$GF_SL'"
fi
GF_CONFLICTS=$(gf_get "sum(1 for n in ('$GF_X', '$GF_Y') if v['back'].get(n) == 'conflict')")
GF_XYL=$(gf_get "sum(1 for n in ('$GF_X', '$GF_Y') if n in v['landed'])")
GF_LOSER=$(gf_get "[n for n in ('$GF_X', '$GF_Y') if v['back'].get(n) == 'conflict'][0]")
GF_SHARED=$(git -C "$GF_ROOT" show war-go/integration:shared.txt 2>/dev/null | tr '\n' ' ')
if [[ "$GF_CONFLICTS" == "1" && "$GF_XYL" == "2" ]] \
    && grep -q "did not land (conflict): merging war-go/integration into its branch conflicted in shared.txt" \
        "$GF_ROOT/docs/tickets/${GF_LOSER%/*}/atoms/10-intent.md" \
    && [[ "$GF_SHARED" == "X " || "$GF_SHARED" == "Y " ]]; then
    gf_ok "a merge conflict is sent back" "$GF_LOSER: conflict in shared.txt, noted; landed again from the new tip; shared.txt is '$GF_SHARED'"
else
    gf_fail "a merge conflict is sent back" "$GF_CONFLICTS conflicts, $GF_XYL landed, shared '$GF_SHARED'"
fi
GF_CB=$(gf_get "v['back'].get('$GF_C')"); GF_CL=$(gf_get "v['landed'].get('$GF_C')")
GF_CHECKRUN=$(grep -c '"type":"ticket.check_run"' "$GF_ROOT/docs/tickets/${GF_C%/*}/journal.jsonl")
if [[ "$GF_CB" == "check-failed" && "$GF_CL" == "[2, 'observed']" && "$GF_CHECKRUN" == "1" ]] \
    && grep -q '^- \[x\] c1 .*\[observed\]' "$GF_ROOT/docs/tickets/${GF_C%/*}/atoms/15-checklist.md"; then
    gf_ok "a failed check ticks nothing" "$GF_C: check-failed (journalled), then observed when its test passed"
else
    gf_fail "a failed check ticks nothing" "back '$GF_CB', landed '$GF_CL', $GF_CHECKRUN check runs"
fi

# P: set aside for a person, open, unclaimed, and listed by `war next`.
GF_PB=$(gf_get "'$GF_P' in v['blocked']")
GF_COMMON=$(git -C "$GF_ROOT" rev-parse --path-format=absolute --git-common-dir)
GF_LOCKS=$(command ls "$GF_COMMON/openwarrant/claims" 2>/dev/null | grep -c '\.lock$')
GF_NEXT=$(gfw --json next 2>/dev/null)
GF_NB=$(gf_field "$GF_NEXT" '[b["command"] for b in v["result"].get("blocked", [])]')
GF_NREADY=$(gf_field "$GF_NEXT" 'sum(1 for r in v["result"].get("ready", []) if r["ticket"] == "'"${GF_P%/*}"'")')
GF_NH=$(gfw next 2>/dev/null)
if [[ "$GF_PB" == "True" && "$GF_LOCKS" == "0" && "$GF_NB" == "['war evidence go --retry $GF_P']" && "$GF_NREADY" == "0" ]] \
    && grep -q '^- \[ \] p1' "$GF_ROOT/docs/tickets/${GF_P%/*}/atoms/15-checklist.md" \
    && grep -q "^HUMAN  $GF_P" <<<"$GF_NH"; then
    gf_ok "attempts used up: set aside" "$GF_P open, no claim held, in war next for a person"
else
    gf_fail "attempts used up: set aside" "blocked '$GF_PB', $GF_LOCKS locks, next '$GF_NB', ready $GF_NREADY"
fi
GF_R1=$(gfw --json evidence go --retry "$GF_P" 2>/dev/null); GF_R1C=$?
GF_R2=$(gfw --json evidence go --retry "$GF_P" 2>/dev/null); GF_R2C=$?
GF_R2R=$(gf_field "$GF_R2" '",".join(d["rule"] for d in v["diagnostics"] if d["severity"] == "error")')
GF_NB=$(gf_field "$(gfw --json next 2>/dev/null)" 'len(v["result"].get("blocked", []))')
if [[ $GF_R1C -eq 0 && $GF_R2C -eq 2 && "$GF_R2R" == "go.not-blocked" && "$GF_NB" == "0" ]]; then
    gf_ok "a person puts it back" "--retry clears it from war next; a second --retry is go.not-blocked"
else
    gf_fail "a person puts it back" "retry exit $GF_R1C, again $GF_R2C '$GF_R2R', still blocked $GF_NB"
fi

command rm -rf "$GF_TMP"
unset GF_ROOT GF_TMP GF_WAR GF_FIX GF_K GF_S GF_X GF_Y GF_C GF_P GF_OUT GF_RC GF_SUM GF_KB GF_KL GF_SB GF_SL \
    GF_CONFLICTS GF_XYL GF_LOSER GF_SHARED GF_CB GF_CL GF_CHECKRUN GF_PB GF_COMMON GF_LOCKS GF_NEXT GF_NB \
    GF_NREADY GF_NH GF_R1 GF_R1C GF_R2 GF_R2C GF_R2R
unset -f gf_ok gf_fail gfw gf_field gf_new gf_get
