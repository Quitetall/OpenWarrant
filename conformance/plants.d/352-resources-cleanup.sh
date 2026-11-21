# shellcheck shell=bash
# `war cleanup` (OW-ADR-0037): the everyday prune, with branches.
#
# Each claim beside its refusal:
# - The dry run lists a merged, clean, managed worktree and a merged branch
#   nothing has checked out, and removes nothing.
# - The plugin's Stop hook says the leftovers in one line, exit 0; the same
#   line is not said twice in a session.
# - `--apply` removes that worktree and its branch, and deletes the merged
#   branch. Refused: a dirty managed worktree, an unmanaged one, and an
#   unmerged branch are kept, each with its reason. On a host that hides a
#   same-user process, activity is UNKNOWN and the just-finished worktree is
#   kept too (a file changed within `idle_after`): that is said, never a removal.
# - Where the process probe is UNKNOWN (forced with the debug build's
#   OPENWARRANT_TEST_PROCESS_PROBE=hidden, so this holds on any host), a
#   clean, merged, managed worktree nothing in which changed for `idle_after`
#   (24h by default) is removed by `--apply` with its branch, the row naming
#   the signal. Refused: the same one with one fresh file is kept, naming it;
#   `idle_after = "0"` keeps it; a dirty and an unmanaged one, aged too, are
#   kept for their own reasons.

echo "== resources: war cleanup (OW-ADR-0037) =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID OPENCODE_SESSION_ID OPENCODE
RC_WAR="$REPO_ROOT/${WAR#./}"
rc_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
RC_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$RC_T")
RC_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid GIT_COMMITTER_NAME=plant
    GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
rc_in() { local d=$1; shift; (cd "$d" && "${RC_ENV[@]}" "$@" </dev/null 2>&1); }
rc_id() { sed -n 's/^\([a-z][a-z]*-[0-9a-f][0-9a-f]*\)  .*/\1/p' <<<"$1" | head -n 1; }
RC_R="$RC_T/repo" && mkdir -p "$RC_R"
rc_in "$RC_R" git init -q -b main >/dev/null && rc_in "$RC_R" "$RC_WAR" init >/dev/null \
    || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
RC_DONE=$(rc_id "$(rc_in "$RC_R" "$RC_WAR" create "finished work")")
RC_DIRTY=$(rc_id "$(rc_in "$RC_R" "$RC_WAR" create "unfinished work")")
rc_in "$RC_R" git add -A >/dev/null && rc_in "$RC_R" git commit -qm plan >/dev/null
[[ -n "$RC_DONE" && -n "$RC_DIRTY" ]] || { printf 'PLANT SETUP FAILED: items\n' >&2; exit 9; }
RC_W1="$RC_R/.openwarrant/worktrees/$RC_DONE"
RC_W2="$RC_R/.openwarrant/worktrees/$RC_DIRTY"
rc_in "$RC_R" "$RC_WAR" start --worktree "$RC_DONE" --print >/dev/null
(cd "$RC_W1" && echo done > done.txt && "${RC_ENV[@]}" "$RC_WAR" done "$RC_DONE" --note ok \
    && "${RC_ENV[@]}" git add -A && "${RC_ENV[@]}" git commit -qm finished) >/dev/null 2>&1
rc_in "$RC_R" git merge -q --no-edit "war-go/$RC_DONE" >/dev/null
rc_in "$RC_R" "$RC_WAR" start --worktree "$RC_DIRTY" --print >/dev/null
echo wip > "$RC_W2/wip.txt"
rc_in "$RC_R" git worktree add -q "$RC_T/foreign" -b war-go/foreign >/dev/null
rc_in "$RC_R" git branch war-go/merged >/dev/null
rc_in "$RC_R" git checkout -q -b war-go/side >/dev/null && echo s > "$RC_R/side.txt" \
    && rc_in "$RC_R" git add side.txt >/dev/null && rc_in "$RC_R" git commit -qm side >/dev/null \
    && rc_in "$RC_R" git checkout -q main >/dev/null
[[ -d "$RC_W1" && -d "$RC_W2" && -d "$RC_T/foreign" ]] || { printf 'PLANT SETUP FAILED: worktrees\n' >&2; exit 9; }

dry=$(rc_in "$RC_R" "$RC_WAR" cleanup); drc=$?
RC_UNKNOWN=0
grep -q "^keep    .openwarrant/worktrees/$RC_DONE: process activity unknown; .* changed .* ago, within idle_after 24h$" <<<"$dry" && RC_UNKNOWN=1
if [[ $drc -eq 0 && "$dry" == *"delete  war-go/merged"* && "$dry" == *"dry run:"* \
    && ( $RC_UNKNOWN -eq 1 || "$dry" == *"remove  .openwarrant/worktrees/$RC_DONE"* ) \
    && -d "$RC_W1" ]] && rc_in "$RC_R" git rev-parse -q --verify refs/heads/war-go/merged >/dev/null; then
    rc_ok "the dry run lists, removes nothing" "war-go/merged$( ((RC_UNKNOWN)) || echo " and .openwarrant/worktrees/$RC_DONE")"
else
    rc_fail "the dry run lists, removes nothing" "exit $drc: $(tr "\n" "|" <<<"$dry")"
fi

# The Stop hook says the leftovers in one line, once per session; never a block.
rc_hook() { printf '{"cwd":"%s","session_id":"plant352"}' "$RC_R" \
    | (cd "$RC_R" && "${RC_ENV[@]}" PATH="$(dirname "$RC_WAR"):$PATH" bash "$REPO_ROOT/.claude/hooks/stop-check.sh" 2>/dev/null); }
h1=$(rc_hook); h1rc=$?
h2=$(rc_hook); h2rc=$?
if [[ $h1rc -eq 0 && $h2rc -eq 0 && "$(grep -c . <<<"$h1")" -eq 1 && "$h1" == *'"systemMessage"'*"leftovers: "*"merged war-go branch"* && -z "$h2" ]]; then
    rc_ok "the Stop hook says leftovers once" "one systemMessage line, exit 0; silent the second time"
else
    rc_fail "the Stop hook says leftovers once" "exit $h1rc/$h2rc: $h1 / $h2"
fi

app=$(rc_in "$RC_R" "$RC_WAR" cleanup --apply); arc=$?
has() { rc_in "$RC_R" git rev-parse -q --verify "refs/heads/$1" >/dev/null; }
if [[ $arc -eq 0 && "$app" == *"deleted war-go/merged"* ]] && ! has war-go/merged; then
    if ((RC_UNKNOWN)); then
        [[ -d "$RC_W1" && "$app" == *"keep    .openwarrant/worktrees/$RC_DONE: process activity unknown"* ]] \
            && rc_ok "--apply removes the finished" "war-go/merged deleted; the clean worktree kept: activity UNKNOWN on this host" \
            || rc_fail "--apply removes the finished" "$(tr "\n" "|" <<<"$app")"
    else
        [[ ! -e "$RC_W1" && "$app" == *"removed .openwarrant/worktrees/$RC_DONE"* && "$app" == *"deleted war-go/$RC_DONE"* ]] \
            && ! has "war-go/$RC_DONE" \
            && rc_ok "--apply removes the finished" "the worktree, war-go/$RC_DONE and war-go/merged" \
            || rc_fail "--apply removes the finished" "$(tr "\n" "|" <<<"$app")"
    fi
else
    rc_fail "--apply removes the finished" "exit $arc: $app"
fi
if [[ -f "$RC_W2/wip.txt" && -d "$RC_T/foreign" ]] && has war-go/side && has "war-go/$RC_DIRTY" && has war-go/foreign \
    && grep -q "^keep    .openwarrant/worktrees/$RC_DIRTY: .*dirty" <<<"$app" \
    && grep -q "^keep    $RC_T/foreign: foreign or ownership unknown" <<<"$app" \
    && grep -q "^keep    war-go/side: not merged into HEAD" <<<"$app"; then
    rc_ok "dirty, unmanaged, unmerged kept" "each with its reason"
else
    rc_fail "dirty, unmanaged, unmerged kept" "$app"
fi

# The second signal: nothing changed for `idle_after`, where processes are hidden.
RC_IDLE=$(rc_id "$(rc_in "$RC_R" "$RC_WAR" create "idle work")")
rc_in "$RC_R" git add -A >/dev/null && rc_in "$RC_R" git commit -qm idle >/dev/null
RC_W3="$RC_R/.openwarrant/worktrees/$RC_IDLE"
rc_in "$RC_R" "$RC_WAR" start --worktree "$RC_IDLE" --print >/dev/null
(cd "$RC_W3" && echo idle > idle.txt && "${RC_ENV[@]}" "$RC_WAR" done "$RC_IDLE" --note ok \
    && "${RC_ENV[@]}" git add -A && "${RC_ENV[@]}" git commit -qm idle) >/dev/null 2>&1
rc_in "$RC_R" git merge -q --no-edit "war-go/$RC_IDLE" >/dev/null
[[ -n "$RC_IDLE" && -f "$RC_W3/idle.txt" ]] && rc_in "$RC_R" git merge-base --is-ancestor "war-go/$RC_IDLE" HEAD \
    || { printf 'PLANT SETUP FAILED: idle worktree\n' >&2; exit 9; }
rc_age() { local d; for d in "$@"; do find "$d" -path "$d/.git" -prune -o -exec touch -h -d "@$(($(date +%s) - 3 * 86400))" {} + ; done; }
rc_hidden() { rc_in "$RC_R" env OPENWARRANT_TEST_PROCESS_PROBE=hidden "$RC_WAR" cleanup "$@"; }
rc_age "$RC_W3" "$RC_W2" "$RC_T/foreign"
RC_I=".openwarrant/worktrees/$RC_IDLE"
d1=$(rc_hidden); d1rc=$?
if [[ $d1rc -eq 0 && -d "$RC_W3" ]] && grep -qx "remove  $RC_I: activity unknown; idle 3d, past 24h" <<<"$d1"; then
    rc_ok "idle past idle_after: eligible" "the dry run names the signal, removes nothing"
else
    rc_fail "idle past idle_after: eligible" "exit $d1rc: $(tr "\n" "|" <<<"$d1")"
fi
touch "$RC_W3/idle.txt"
d2=$(rc_hidden); d2rc=$?
if [[ $d2rc -eq 0 ]] && grep -qx "keep    $RC_I: process activity unknown; idle.txt changed 0m ago, within idle_after 24h" <<<"$d2"; then
    rc_ok "a fresh file keeps it" "idle.txt named"
else
    rc_fail "a fresh file keeps it" "exit $d2rc: $(tr "\n" "|" <<<"$d2")"
fi
rc_age "$RC_W3"
cp "$RC_R/openwarrant.toml" "$RC_T/config.bak"
printf '\n[resources]\nidle_after = "0"\n' >> "$RC_R/openwarrant.toml"
d3=$(rc_hidden); d3rc=$?
/usr/bin/cp -f "$RC_T/config.bak" "$RC_R/openwarrant.toml"
if [[ $d3rc -eq 0 ]] && grep -qx "keep    $RC_I: process activity unknown; idle_after 0" <<<"$d3"; then
    rc_ok "idle_after = 0 keeps it" "the fallback is off"
else
    rc_fail "idle_after = 0 keeps it" "exit $d3rc: $(tr "\n" "|" <<<"$d3")"
fi
a2=$(rc_hidden --apply); a2rc=$?
if [[ $a2rc -eq 0 && ! -e "$RC_W3" ]] && ! has "war-go/$RC_IDLE" \
    && grep -qx "removed $RC_I: activity unknown; idle 3d, past 24h" <<<"$a2" \
    && grep -qx "deleted war-go/$RC_IDLE" <<<"$a2"; then
    rc_ok "--apply removes the idle one" "the worktree and war-go/$RC_IDLE"
else
    rc_fail "--apply removes the idle one" "exit $a2rc: $(tr "\n" "|" <<<"$a2")"
fi
if [[ -f "$RC_W2/wip.txt" && -d "$RC_T/foreign" ]] && has "war-go/$RC_DIRTY" && has war-go/foreign \
    && grep -q "^keep    .openwarrant/worktrees/$RC_DIRTY: .*dirty" <<<"$a2" \
    && grep -q "^keep    $RC_T/foreign: foreign or ownership unknown" <<<"$a2"; then
    rc_ok "aged dirty and unmanaged kept" "idle is never enough alone"
else
    rc_fail "aged dirty and unmanaged kept" "$(tr "\n" "|" <<<"$a2")"
fi
