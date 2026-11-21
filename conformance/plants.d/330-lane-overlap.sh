# shellcheck shell=bash
# Lanes do not overlap while they run at the same time (OW-ADR-0036).
#
# Accepted: a lane disjoint from the others; a lane sharing another's files
#   when it runs after it (`--after`); `war check` passes them (lane.disjoint).
# Refused: a lane whose paths overlap a lane running at the same time
#   (lane.overlap at `war lane new`, nothing written); a lane document edited
#   by hand to overlap one beside it is an error in `war check` (lane.overlap),
#   naming both lanes.

echo "== lanes running at the same time never overlap =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID OPENCODE_SESSION_ID OPENCODE
LO_WAR="$REPO_ROOT/${WAR#./}"
lo_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
lo_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
LO_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$LO_T")
LO_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid GIT_COMMITTER_NAME=plant
    GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
lo_war() { (cd "$LO_T" && "${LO_ENV[@]}" "$LO_WAR" "$@" </dev/null 2>/dev/null); }
lo_rules() { python3 -c 'import json,sys
try: d=json.load(sys.stdin)
except Exception: sys.exit(0)
print(" ".join(x.get("rule","") for x in d.get("diagnostics",[]) if x.get("severity") in ("error","unknown")))'; }
mkdir -p "$LO_T/src/a" "$LO_T/src/d" && echo x >"$LO_T/src/a/x.rs"
(cd "$LO_T" && "${LO_ENV[@]}" git init -q -b main && "${LO_ENV[@]}" "$LO_WAR" init >/dev/null 2>&1 \
    && "${LO_ENV[@]}" git add -A && "${LO_ENV[@]}" git commit -qm seed) \
    || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
# Work lanes only: the cleanup lane every lane plan ends in (OW-ADR-0037) is
# not one of them.
lo_count() { grep -rl '^war.lane: true' "$LO_T/docs" 2>/dev/null | xargs -r grep -L '^# cleanup$' | wc -l | tr -d ' '; }

out=$(lo_war --json lane new api --paths src/a)
[[ $(lo_rules <<<"$out") == "" && $(lo_count) == 1 ]] && lo_ok "lane new" "a lane is written" \
    || lo_fail "lane new" "$(head -c 300 <<<"$out")"
grep -rq '^  - "war.lane"' "$LO_T/docs" && lo_ok "lane requires" "requires lists war.lane" \
    || lo_fail "lane requires" "no requires: [war.lane]"
out=$(lo_war --json lane new beside --paths src/a/x.rs)
if [[ $(lo_rules <<<"$out") == "lane.overlap" && $(lo_count) == 1 ]] && grep -q 'api' <<<"$out"; then
    lo_ok "overlap refused" "lane.overlap names api; nothing written"
else
    lo_fail "overlap refused" "$(lo_rules <<<"$out") count=$(lo_count)"
fi
out=$(lo_war --json lane new after --paths src/a/x.rs --after api)
[[ $(lo_rules <<<"$out") == "" && $(lo_count) == 2 ]] && lo_ok "in sequence allowed" "--after api may share src/a" \
    || lo_fail "in sequence allowed" "$(head -c 300 <<<"$out")"
out=$(lo_war --json lane new db --paths src/d)
[[ $(lo_rules <<<"$out") == "" && $(lo_count) == 3 ]] && lo_ok "disjoint allowed" "src/d beside src/a" \
    || lo_fail "disjoint allowed" "$(head -c 300 <<<"$out")"
out=$(lo_war --json check)
if grep -q '"lane.disjoint"' <<<"$out" && ! grep -q '"lane.overlap"' <<<"$out"; then
    lo_ok "check passes" "lane.disjoint"
else
    lo_fail "check passes" "$(grep -o '"rule": "lane[^"]*"' <<<"$out" | tr '\n' ' ')"
fi
# By hand: db's paths now reach into api's.
LO_DB=$(grep -rl '^war.lane: true' "$LO_T/docs" | xargs grep -l '^# db$' | head -1)
python3 - "$LO_DB" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace('  - "src/d/**"\n', '  - "src/d/**"\n  - "src/**"\n', 1)
open(p, "w").write(s)
PY
out=$(lo_war --json check)
if grep -q '"rule": "lane.overlap"' <<<"$out" && grep -q 'api' <<<"$out" && grep -q 'db' <<<"$out"; then
    lo_ok "check refuses" "lane.overlap names api and db"
else
    lo_fail "check refuses" "$(grep -o '"rule": "lane[^"]*"' <<<"$out" | tr '\n' ' ')"
fi
