# shellcheck shell=bash
# OW-WAR-0148 M14 — telling a person something waits on them: `[notify]`,
# `war next` and the stop hook's end-of-session summary (docs/PRESETS.md).
#
# A team repository with a notify command written here, which records its
# argv and the four OPENWARRANT_NOTIFY_* variables. Each claim is paired
# with a refusal:
#
#   1. `war sign approve <id>` (the request) runs [notify] once, with the
#      event, subject and command filled in; asked again for the same plan
#      it records nothing more and runs nothing. Refused to run: without
#      [notify], nothing is started.
#   2. a notify that fails is reported (notify.failed, a warning) and blocks
#      nothing: exit 0, the request recorded and listed by `war next`.
#   3. the Claude Code stop hook, under a preset, shows the person what
#      waits (a systemMessage naming the approval); in a repository with no
#      preset it says nothing.

echo "== notifications (M14) =="
NT_T=$(mktemp -d)
NT_ROOT="$NT_T/repo"
NT_WAR="$REPO_ROOT/${WAR#./}"
nt_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
nt_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ntw() { (cd "$NT_ROOT" && env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
    OPENWARRANT_NO_PROJECTS=1 "$NT_WAR" "$@" </dev/null); }
nt_py() { python3 -c "import json, sys; v = json.load(sys.stdin); print($1)" 2>/dev/null; }

mkdir -p "$NT_ROOT"
git -C "$NT_ROOT" init -q .
ntw init --team >/dev/null 2>&1
cat > "$NT_T/notify.sh" <<'SH'
#!/bin/sh
printf 'argv=%s|env=%s %s %s\n' "$*" "$OPENWARRANT_NOTIFY_EVENT" "$OPENWARRANT_NOTIFY_SUBJECT" \
    "$OPENWARRANT_NOTIFY_COMMAND" >> "$NT_LOG"
exit "${NT_EXIT:-0}"
SH
chmod +x "$NT_T/notify.sh"
NT_LOG="$NT_T/notify.log"
export NT_LOG
: > "$NT_LOG"

# ---- 1. no [notify], then [notify] ------------------------------------------------------
NT_A=$(ntw --json create "Fix the redirect" | nt_py 'v["result"]["id"]')
ntw sign approve "$NT_A" >/dev/null 2>&1; NT_OFF_RC=$?
NT_OFF_LOG=$(cat "$NT_LOG")
printf '\n[notify]\nargv = ["%s", "{event}", "{subject}"]\n' "$NT_T/notify.sh" >> "$NT_ROOT/openwarrant.toml"
NT_B=$(ntw --json create "Cache the session" | nt_py 'v["result"]["id"]')
NT_ON=$(ntw --json sign approve "$NT_B"); NT_ON_RC=$?
NT_ON_LOG=$(cat "$NT_LOG")
ntw sign approve "$NT_B" >/dev/null 2>&1
NT_TWICE=$(grep -c . "$NT_LOG")
if [[ $NT_ON_RC -eq 0 && "$NT_TWICE" == "1" ]] \
    && [[ "$NT_ON_LOG" == "argv=approval.requested $NT_B|env=approval.requested $NT_B war sign approve $NT_B --ssh-sign" ]] \
    && [[ "$(nt_py '",".join(d["rule"] for d in v["diagnostics"])' <<<"$NT_ON")" == "notify.sent" ]]; then
    nt_ok "a request runs [notify] once" "argv and variables filled; the same plan asked again runs nothing"
else
    nt_fail "a request runs [notify] once" "exit $NT_ON_RC, runs $NT_TWICE: '$NT_ON_LOG'"
fi
if [[ $NT_OFF_RC -eq 0 && -z "$NT_OFF_LOG" ]]; then
    nt_ok "without [notify] nothing runs" "the request recorded; no command started"
else
    nt_fail "without [notify] nothing runs" "exit $NT_OFF_RC, log '$NT_OFF_LOG'"
fi

# ---- 2. a failing notify blocks nothing -----------------------------------------------------
NT_C=$(ntw --json create "Guard the empty password" | nt_py 'v["result"]["id"]')
NT_FAILED=$( (export NT_EXIT=3; ntw sign approve "$NT_C" 2>&1) ); NT_FAILED_RC=$?
NT_NEXT=$(ntw next 2>&1)
if [[ $NT_FAILED_RC -eq 0 ]] && grep -qF 'notify.failed' <<<"$NT_FAILED" \
    && grep -qF "approval requested for $NT_C" <<<"$NT_FAILED" \
    && line_has -F "$NT_C" -F 'war sign approve' <<<"$NT_NEXT"; then
    nt_ok "a failing notify blocks nothing" "notify.failed reported; exit 0; the request recorded and listed by war next"
else
    nt_fail "a failing notify blocks nothing" "exit $NT_FAILED_RC: $(tr '\n' '|' <<<"$NT_FAILED" | head -c 300)"
fi

# ---- 3. the stop hook's summary -----------------------------------------------------------------
nt_hook() { # repo -> the hook's stdout
    printf '{"session_id":"plant","hook_event_name":"Stop","stop_hook_active":false,"cwd":"%s"}' "$1" \
        | PATH="$REPO_ROOT/target/debug:$PATH" bash "$REPO_ROOT/.claude/hooks/stop-check.sh" 2>/dev/null
}
NT_HOOK=$(nt_hook "$NT_ROOT")
NT_MSG=$(nt_py 'v["systemMessage"]' <<<"$NT_HOOK")
NT_PLAIN="$NT_T/plain"
mkdir -p "$NT_PLAIN"; git -C "$NT_PLAIN" init -q .
(cd "$NT_PLAIN" && env -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 "$NT_WAR" init >/dev/null 2>&1)
(cd "$NT_PLAIN" && env -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 "$NT_WAR" create "Plain work" >/dev/null 2>&1)
NT_PLAIN_HOOK=$(nt_hook "$NT_PLAIN")
if [[ "$NT_MSG" == "OpenWarrant: 3 act(s) wait on a person (war next lists them): approve "* ]] \
    && grep -qF "approve $NT_B" <<<"$NT_MSG" && [[ -z "$NT_PLAIN_HOOK" ]]; then
    nt_ok "the stop hook names what waits" "a systemMessage naming the approvals; nothing in a repository with no preset"
else
    nt_fail "the stop hook names what waits" "'$NT_MSG' | plain '$NT_PLAIN_HOOK'"
fi

unset -f nt_ok nt_fail ntw nt_py nt_hook
unset NT_LOG
command rm -rf "$NT_T"
