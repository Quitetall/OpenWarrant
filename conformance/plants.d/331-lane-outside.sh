# shellcheck shell=bash
# A lane's work stays in its paths (OW-ADR-0036).
#
# Refused: `war done` on a lane item after a change outside the lane's paths
#   since the claim's base revision (committed, tracked or untracked), naming
#   each file, with nothing written (document and journal byte-identical);
#   `war lane widen` into the paths of a lane running at the same time
#   (lane.overlap), nothing written.
# Accepted: after `war lane widen` (journalled `lane.widened` with its actor)
#   and the other file undone, the same done passes. An item outside every
#   lane with paths keeps today's meaning: its done is never refused for them.

echo "== a lane's work stays in its paths =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID
LP_WAR="$REPO_ROOT/${WAR#./}"
lp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
lp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
LP_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$LP_T")
LP_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid GIT_COMMITTER_NAME=plant
    GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
lp_war() { (cd "$LP_T" && "${LP_ENV[@]}" "$LP_WAR" "$@" </dev/null 2>/dev/null); }
lp_git() { (cd "$LP_T" && "${LP_ENV[@]}" git "$@" >/dev/null 2>&1); }
lp_id() { python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])' 2>/dev/null; }
lp_rules() { python3 -c 'import json,sys
try: d=json.load(sys.stdin)
except Exception: sys.exit(0)
print(" ".join(x.get("rule","") for x in d.get("diagnostics",[]) if x.get("severity") in ("error","unknown")))'; }
mkdir -p "$LP_T/src/api" "$LP_T/src/db" && echo a >"$LP_T/src/api/a.rs" && echo d >"$LP_T/src/db/d.rs"
(cd "$LP_T" && "${LP_ENV[@]}" git init -q -b main && "${LP_ENV[@]}" "$LP_WAR" init >/dev/null 2>&1) \
    || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
lp_war lane new api --paths src/api >/dev/null
lp_war lane new db --paths src/db >/dev/null
LP_I=$(lp_war --json create "api work" --lane api | lp_id)
LP_FREE=$(lp_war --json create "free work" --path src/api/a.rs | lp_id)
lp_git add -A && lp_git commit -qm seed
[[ -n "$LP_I" && -n "$LP_FREE" ]] || { printf 'PLANT SETUP FAILED: items\n' >&2; exit 9; }
LP_DOC=$(grep -rl "^alias: $LP_I\$" "$LP_T/docs" | head -1)
LP_LANE=$(grep -rl '^war.lane: true' "$LP_T/docs" | xargs grep -l '^# api$' | head -1)
lp_state() { cat "$LP_DOC" "$LP_LANE" $(find "$LP_T/docs" -name 'journal*' -type f | sort) 2>/dev/null | sha256sum; }
lp_war start --lane api >/dev/null
echo b >>"$LP_T/src/api/a.rs"
echo e >>"$LP_T/src/db/d.rs" && lp_git commit -qam "outside, committed"
echo n >"$LP_T/new.txt"
before=$(lp_state)
out=$(lp_war --json done "$LP_I")
if [[ $(lp_rules <<<"$out") == "lane.outside-paths" ]] && grep -q 'src/db/d.rs' <<<"$out" \
    && grep -q 'new.txt' <<<"$out" && ! grep -q 'src/api/a.rs' <<<"$out" && [[ $(lp_state) == "$before" ]]; then
    lp_ok "done outside refused" "names src/db/d.rs and new.txt; nothing written"
else
    lp_fail "done outside refused" "$(lp_rules <<<"$out") $(grep -o 'changed outside[^"]*' <<<"$out" | head -c 200)"
fi
out=$(lp_war --json lane widen api 'src/db/**')
if [[ $(lp_rules <<<"$out") == "lane.overlap" ]] && grep -q 'db' <<<"$out" && [[ $(lp_state) == "$before" ]]; then
    lp_ok "widen into a live lane refused" "lane.overlap; nothing written"
else
    lp_fail "widen into a live lane refused" "$(lp_rules <<<"$out")"
fi
out=$(lp_war --json lane widen api new.txt)
LP_W=$(grep -rh '"lane.widened"' "$LP_T/docs")
if [[ $(lp_rules <<<"$out") == "" && "$LP_W" == *'"runner"'* ]] && grep -q '"new.txt"' "$LP_LANE"; then
    lp_ok "widen" "lane.widened journalled by runner; new.txt in paths"
else
    lp_fail "widen" "$(head -c 300 <<<"$out")"
fi
out=$(lp_war --json done "$LP_I")
LP_SAID=$(sed -n 's/.*changed outside it since [^:]*: \(.*\)\. Undo.*/\1/p' <<<"$out" | head -1)
[[ $(lp_rules <<<"$out") == "lane.outside-paths" && "$LP_SAID" == *src/db/d.rs* && "$LP_SAID" != *new.txt* ]] \
    && lp_ok "still outside" "src/db/d.rs alone" || lp_fail "still outside" "$(lp_rules <<<"$out")"
lp_git revert --no-edit HEAD
out=$(lp_war --json done "$LP_I")
[[ $(lp_rules <<<"$out") == "" ]] && lp_ok "done after widen" "passes" || lp_fail "done after widen" "$(head -c 300 <<<"$out")"
# Outside a lane, paths are coordination only.
lp_war start "$LP_FREE" >/dev/null
echo z >>"$LP_T/src/db/d.rs"
out=$(lp_war --json done "$LP_FREE")
[[ $(lp_rules <<<"$out") == "" ]] && lp_ok "no lane, no refusal" "item paths stay coordination" \
    || lp_fail "no lane, no refusal" "$(lp_rules <<<"$out")"
