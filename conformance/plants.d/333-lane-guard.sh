# shellcheck shell=bash
# The plugin's edit guard holds a lane runner to the lane's paths (OW-ADR-0036).
#
# Refused: holding an item of a lane, an edit outside the lane's paths is
#   denied, naming the lane.
# Accepted: an edit inside them, and the records `war` writes; with no lane
#   item held, the guard says nothing about lanes.

echo "== the edit guard holds a lane runner to its paths =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID OPENCODE_SESSION_ID OPENCODE
LG_WAR="$REPO_ROOT/${WAR#./}"
LG_HOOK="$REPO_ROOT/.claude/hooks/guard-pins.sh"
lg_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
lg_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
LG_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$LG_T")
LG_BIN="$LG_T/bin" && mkdir -p "$LG_BIN" && ln -s "$LG_WAR" "$LG_BIN/war"
LG_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner PATH="$LG_BIN:$PATH"
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid GIT_COMMITTER_NAME=plant
    GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
LG_R="$LG_T/repo" && mkdir -p "$LG_R/src/api" "$LG_R/src/db"
lg_war() { (cd "$LG_R" && "${LG_ENV[@]}" "$LG_WAR" "$@" </dev/null 2>/dev/null); }
lg_hook() { printf '{"tool_input":{"file_path":"%s"},"cwd":"%s"}' "$LG_R/$1" "$LG_R" \
    | (cd "$LG_R" && "${LG_ENV[@]}" bash "$LG_HOOK" 2>/dev/null); }
(cd "$LG_R" && "${LG_ENV[@]}" git init -q -b main && "${LG_ENV[@]}" "$LG_WAR" init >/dev/null 2>&1) \
    || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
lg_war lane new api --paths src/api >/dev/null
lg_war create "api work" --lane api >/dev/null
out=$(lg_hook src/db/x.rs)
[[ "$out" != *'"deny"'* ]] && lg_ok "no lane held" "silent" || lg_fail "no lane held" "$out"
lg_war start --lane api >/dev/null
out=$(lg_hook src/db/x.rs)
[[ "$out" == *'"deny"'* && "$out" == *'lane api'* && "$out" == *'src/db/x.rs'* ]] \
    && lg_ok "outside denied" "names lane api and the file" || lg_fail "outside denied" "$out"
out=$(lg_hook src/api/x.rs)
[[ "$out" != *'"deny"'* ]] && lg_ok "inside allowed" "src/api/x.rs" || lg_fail "inside allowed" "$out"
out=$(lg_hook docs/tickets/x.md)
[[ "$out" != *'"deny"'* ]] && lg_ok "records allowed" "docs/tickets" || lg_fail "records allowed" "$out"
