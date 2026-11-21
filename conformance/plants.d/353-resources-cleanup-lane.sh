# shellcheck shell=bash
# The cleanup pass is a lane (OW-ADR-0037).
#
# Each claim beside its refusal:
# - A lane plan gets one `cleanup` lane, after every work lane, with three
#   items; a third work lane joins its `depends_on` and adds no second lane
#   and no item (idempotent). `war check` passes over it. Refused:
#   `[lanes] cleanup = false` gives no cleanup lane.
# - A lane's brief names its `war.reuse` paths (`Reuse: …`); a lane without
#   them says nothing of reuse. The cleanup lane's brief recommends a
#   different runner.

echo "== resources: the cleanup lane (OW-ADR-0037) =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID OPENCODE_SESSION_ID OPENCODE
CL_WAR="$REPO_ROOT/${WAR#./}"
cl_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cl_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
CL_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$CL_T")
CL_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid GIT_COMMITTER_NAME=plant
    GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
cl_in() { local d=$1; shift; (cd "$d" && "${CL_ENV[@]}" "$@" </dev/null 2>&1); }
cl_repo() { mkdir -p "$1" && cl_in "$1" git init -q -b main >/dev/null && cl_in "$1" "$CL_WAR" init >/dev/null; }
cl_lanes() { cl_in "$1" "$CL_WAR" status | grep '^lane '; }

CL_R="$CL_T/repo"
cl_repo "$CL_R" || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
CL_A=$(cl_in "$CL_R" "$CL_WAR" lane new api --paths src/api --reuse src/core/registry.rs | sed -n 's/^lane api \([^ ]*\) .*/\1/p')
CL_B=$(cl_in "$CL_R" "$CL_WAR" lane new db --paths src/db | sed -n 's/^lane db \([^ ]*\) .*/\1/p')
first=$(cl_lanes "$CL_R" | grep -c '^lane cleanup ')
CL_C=$(cl_in "$CL_R" "$CL_WAR" lane new web --paths src/web | sed -n 's/^lane web \([^ ]*\) .*/\1/p')
row=$(cl_lanes "$CL_R" | grep '^lane cleanup ')
n=$(grep -c . <<<"$row")
cl_in "$CL_R" "$CL_WAR" check >/dev/null; crc=$?
if [[ -n "$CL_A" && -n "$CL_B" && -n "$CL_C" && $first -eq 1 && $n -eq 1 && "$row" == *" 0/3 "* \
    && "$row" == *"after $CL_A, $CL_B, $CL_C"* && $crc -eq 0 ]]; then
    cl_ok "one cleanup lane, after every lane" "3 items, after $CL_A, $CL_B, $CL_C; a third lane added no second; check passes"
else
    cl_fail "one cleanup lane, after every lane" "first $first, now $n, check $crc: $row"
fi

CL_F="$CL_T/off"
cl_repo "$CL_F" || { printf 'PLANT SETUP FAILED: second repository\n' >&2; exit 9; }
printf '\n[lanes]\ncleanup = false\n' >> "$CL_F/openwarrant.toml"
cl_in "$CL_F" "$CL_WAR" lane new api --paths src/api >/dev/null
cl_in "$CL_F" "$CL_WAR" lane new db --paths src/db >/dev/null
off=$(cl_lanes "$CL_F")
if [[ "$(grep -c . <<<"$off")" -eq 2 && "$off" != *"lane cleanup "* ]]; then
    cl_ok "cleanup = false gives none" "two work lanes, no cleanup lane"
else
    cl_fail "cleanup = false gives none" "$off"
fi

# Briefs: Reuse on api, none on db; the cleanup lane names a different runner.
CL_I=$(cl_in "$CL_R" "$CL_WAR" create "api work" --lane api | sed -n 's/^\([a-z][a-z]*-[0-9a-f][0-9a-f]*\)  .*/\1/p' | head -n 1)
cl_in "$CL_R" "$CL_WAR" create "db work" --lane db >/dev/null
cl_in "$CL_R" "$CL_WAR" create "web work" --lane web >/dev/null
cl_in "$CL_R" git add -A >/dev/null && cl_in "$CL_R" git commit -qm plan >/dev/null
api=$(cl_in "$CL_R" "$CL_WAR" start --lane api)
db=$(cl_in "$CL_R" "$CL_WAR" start --lane db --as other)
if [[ "$api" == *"Reuse: src/core/registry.rs"* && "$db" != *"Reuse:"* ]]; then
    cl_ok "a lane brief names what to reuse" "api says Reuse: src/core/registry.rs; db says nothing"
else
    cl_fail "a lane brief names what to reuse" "api: $(grep -i reuse <<<"$api") db: $(grep -i reuse <<<"$db")"
fi
cl_in "$CL_R" "$CL_WAR" done --as runner >/dev/null
cl_in "$CL_R" "$CL_WAR" done --as other >/dev/null
cl_in "$CL_R" "$CL_WAR" start --lane web --as third >/dev/null
cl_in "$CL_R" "$CL_WAR" done --as third >/dev/null
wait_out=$(cl_in "$CL_R" "$CL_WAR" start --lane cleanup --as cleaner)
if [[ "$wait_out" == *"cleanup lane: best run by a different runner"* && "$wait_out" == *"docs pass"* ]]; then
    cl_ok "the cleanup brief: another runner" "after every lane: docs pass first, a different runner"
else
    cl_fail "the cleanup brief: another runner" "$(head -n 6 <<<"$wait_out" | tr '\n' '|')"
fi
