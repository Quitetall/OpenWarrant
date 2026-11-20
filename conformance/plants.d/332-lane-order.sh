# shellcheck shell=bash
# Lanes run in order, joins wait on lanes, and `--for` filters (OW-ADR-0036).
#
# Accepted: `war create --lane` puts each item last (its `depends_on` the
#   previous last); `war start --lane` takes the first open item and claims
#   the lane's paths; a lane after another starts once it is done; a join
#   (`--depends-on` both lanes) is ready once both are; `ready --for` lists
#   what is for that agent; a lane whose items are all done reads done.
# Refused: `start --lane` on a lane whose prior lane is open (lane.waiting);
#   the second item before the first is done (it is not handed over); the join
#   while one lane is open (not ready); `ready --for codex` never lists
#   claude's work.

echo "== lanes run in order; joins wait; --for filters =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID
LR_WAR="$REPO_ROOT/${WAR#./}"
lr_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
lr_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
LR_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$LR_T")
LR_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid GIT_COMMITTER_NAME=plant
    GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
lr_war() { (cd "$LR_T" && "${LR_ENV[@]}" "$LR_WAR" "$@" </dev/null 2>/dev/null); }
lr_git() { (cd "$LR_T" && "${LR_ENV[@]}" git "$@" >/dev/null 2>&1); }
lr_id() { python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])' 2>/dev/null; }
lr_rules() { python3 -c 'import json,sys
try: d=json.load(sys.stdin)
except Exception: sys.exit(0)
print(" ".join(x.get("rule","") for x in d.get("diagnostics",[]) if x.get("severity") in ("error","unknown")))'; }
lr_ready() { lr_war --json ready "$@" | python3 -c 'import json,sys
print(" ".join(sorted({r["ticket"] for r in json.load(sys.stdin)["result"]["ready"]})))' 2>/dev/null; }
lr_started() { python3 -c 'import json,sys; print(json.load(sys.stdin)["result"].get("ticket") or "")' 2>/dev/null; }
mkdir -p "$LR_T/a" "$LR_T/c" && echo a >"$LR_T/a/a.rs" && echo c >"$LR_T/c/c.rs"
(cd "$LR_T" && "${LR_ENV[@]}" git init -q -b main && "${LR_ENV[@]}" "$LR_WAR" init >/dev/null 2>&1) \
    || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
LR_A=$(lr_war --json lane new alpha --paths a --for claude | lr_id)
LR_B=$(lr_war --json lane new beta --paths a --after alpha | lr_id)
LR_C=$(lr_war --json lane new gamma --paths c --for codex | lr_id)
LR_A1=$(lr_war --json create "alpha one" --lane alpha | lr_id)
LR_A2=$(lr_war --json create "alpha two" --lane alpha | lr_id)
LR_B1=$(lr_war --json create "beta one" --lane beta | lr_id)
LR_C1=$(lr_war --json create "gamma one" --lane gamma | lr_id)
LR_J=$(lr_war --json create "release" --depends-on "$LR_A" --depends-on "$LR_C" | lr_id)
lr_git add -A && lr_git commit -qm seed
[[ -n "$LR_A$LR_B$LR_C" && -n "$LR_A1" && -n "$LR_A2" && -n "$LR_B1" && -n "$LR_C1" && -n "$LR_J" ]] \
    || { printf 'PLANT SETUP FAILED: lanes and items\n' >&2; exit 9; }
LR_A2DOC=$(grep -rl "^alias: $LR_A2\$" "$LR_T/docs" | head -1)
LR_DEP=$(grep -A1 '^depends_on:' "$LR_A2DOC")
[[ "$LR_DEP" == *"$LR_A1"* ]] && lr_ok "create --lane" "$LR_A2 depends on $LR_A1" \
    || lr_fail "create --lane" "no depends_on $LR_A1"

r=$(lr_ready --for claude)
[[ " $r " == *" $LR_A1 "* && " $r " != *" $LR_C1 "* ]] && lr_ok "ready --for claude" "$r" || lr_fail "ready --for claude" "$r"
r=$(lr_ready --for codex)
[[ "$r" == "$LR_C1" ]] && lr_ok "ready --for codex" "claude's work not listed" || lr_fail "ready --for codex" "$r"
r=$(lr_ready)
if [[ " $r " == *" $LR_A1 "* && " $r " == *" $LR_C1 "* && " $r " != *" $LR_A2 "* && " $r " != *" $LR_B1 "* \
    && " $r " != *" $LR_J "* && " $r " != *" $LR_A "* ]]; then
    lr_ok "ready" "first items only; no lane, no join"
else
    lr_fail "ready" "$r"
fi
out=$(lr_war --json start --lane beta)
[[ $(lr_rules <<<"$out") == "lane.waiting" ]] && grep -q alpha <<<"$out" && lr_ok "after waits" "beta waits on alpha" \
    || lr_fail "after waits" "$(lr_rules <<<"$out")"
s=$(lr_war --json start --lane alpha | lr_started)
[[ "$s" == "$LR_A1" ]] && lr_ok "start --lane order" "$LR_A1 first" || lr_fail "start --lane order" "got $s"
LR_CL=$(cat $(find "$LR_T/docs" -path "*/$LR_A1/journal.jsonl") 2>/dev/null)
[[ "$LR_CL" == *'"a/**"'* ]] && lr_ok "claim covers the lane" "a/**" || lr_fail "claim covers the lane" "no a/** in the claim"
s=$(lr_war --json start --lane alpha | lr_started)
[[ "$s" == "$LR_A1" ]] && lr_ok "held is resumed" "not $LR_A2" || lr_fail "held is resumed" "got $s"
lr_war done "$LR_A1" >/dev/null
s=$(lr_war --json start --lane alpha | lr_started)
[[ "$s" == "$LR_A2" ]] && lr_ok "then the second" "$LR_A2" || lr_fail "then the second" "got $s"
lr_war done "$LR_A2" >/dev/null
out=$(lr_war --json plan frontier)
python3 -c 'import json,sys
d=json.load(sys.stdin)["result"]
s={l["name"]:l["state"] for l in d.get("lanes",[])}
j=[x["id"] for x in d.get("joins",[])]
sys.exit(0 if s.get("alpha")=="done" and s.get("gamma")=="open" and sys.argv[1] in j else 1)' "$LR_J" <<<"$out" \
    && lr_ok "frontier" "alpha done, gamma open, the join listed" || lr_fail "frontier" "$(head -c 300 <<<"$out")"
out=$(lr_war status)
grep -q "lane alpha $LR_A  2/2  done" <<<"$out" && lr_ok "status" "alpha reads done" || lr_fail "status" "$(grep lane <<<"$out")"
r=$(lr_ready)
[[ " $r " != *" $LR_J "* && " $r " == *" $LR_B1 "* ]] && lr_ok "join waits on both" "gamma still open; beta free" \
    || lr_fail "join waits on both" "$r"
lr_war start --lane gamma >/dev/null
lr_war done "$LR_C1" >/dev/null
r=$(lr_ready)
[[ " $r " == *" $LR_J "* ]] && lr_ok "join ready" "both lanes done" || lr_fail "join ready" "$r"
