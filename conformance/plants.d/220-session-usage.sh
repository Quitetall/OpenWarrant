# shellcheck shell=bash
# Session usage: `war done` journals what a harness's own record says a work
# session spent (`work.usage`), and Mission Control and `war status --usage`
# show it. docs/design/tui-usage.md. The harness is Claude Code, read from a
# fixture transcript under conformance/fixtures/usage, never a real ~/.claude:
# CLAUDE_CONFIG_DIR points at a throwaway copy. Every session variable is
# cleared, then the one a case needs is set. Each claim beside its refusal:
#
# - A transcript gives exact totals: lines written before `war start` are not
#   counted, a message written as several lines is counted once, and a
#   subagent's transcript is the session's.
# - Message content is never recorded, and the grep that says so is seen to
#   find it in the transcript.
# - A missing transcript is journalled UNKNOWN with its reason, never 0, and
#   `war status --usage` says UNKNOWN.
# - No price table, no dollars; a dated one gives an estimate labelled `est.`
#   with its date.
# - A `prices_as_of` far in the past is shown STALE; a current one is not.
# - `[usage] record = false` records nothing and leaves no cursor.
# - `--usage` takes numbers only: a report with content is refused and ticks
#   nothing; a good one is journalled as `reported`.

echo "== session usage (work.usage) =="
SU_TMP=$(mktemp -d)
SU_WAR="$REPO_ROOT/${WAR#./}"
SU_FIX="$REPO_ROOT/conformance/fixtures/usage"
su_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
su_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
SU_ROOT="$SU_TMP/repo"
SU_CC="$SU_TMP/claude"
SU_SID=0f1c2d3e-0000-4000-8000-0000000a8e20
SU_TX="$SU_CC/projects/-fixture/$SU_SID.jsonl"
mkdir -p "$SU_ROOT" "$SU_CC/projects/-fixture"
git -C "$SU_ROOT" init -q .
# su_war <session id or ""> args...: no session variable but the one given.
su_war() {
    local sid=$1; shift
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID \
        -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID -u WAR_GO_PACKET \
        ${sid:+CLAUDE_CODE_SESSION_ID="$sid"} CLAUDE_CONFIG_DIR="$SU_CC" CODEX_HOME="$SU_TMP/no-codex" \
        HERDR_SOCKET_PATH="$SU_TMP/no.sock" OPENWARRANT_ACTOR=agent "$SU_WAR" --root "$SU_ROOT" "$@"
}
su_new() {  # <title> -> the new ticket's id
    su_war "" --json create "$1" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])'
}
# The work.usage payloads journalled on <ticket>, one JSON per line.
su_usage() {
    python3 - "$SU_ROOT" "$1" <<'PY'
import json, os, sys
root, ticket = sys.argv[1], sys.argv[2]
for dirpath, _, files in os.walk(os.path.join(root, "docs")):
    if "journal.jsonl" in files and os.path.basename(dirpath) == ticket:
        for line in open(os.path.join(dirpath, "journal.jsonl")):
            e = json.loads(line)
            if e.get("type") in ("work.usage", "ticket.usage"):
                p = e["payload"]
                print(json.dumps(p if isinstance(p, dict) else json.loads(p), sort_keys=True))
PY
}
su_war "" init >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: war init\n' >&2; exit 9; }
cp "$SU_FIX/before.jsonl" "$SU_TX"

# Exact totals, from what the session appended after `war start` marked it.
SU_T1=$(su_new "Reset passwords")
[[ -n "$SU_T1" ]] || { printf 'PLANT SETUP FAILED: no ticket\n' >&2; exit 9; }
su_war "$SU_SID" start "$SU_T1" >/dev/null 2>&1
cat "$SU_FIX/after.jsonl" >>"$SU_TX"
mkdir -p "$SU_CC/projects/-fixture/$SU_SID/subagents"
cp "$SU_FIX/subagent.jsonl" "$SU_CC/projects/-fixture/$SU_SID/subagents/agent-fixture.jsonl"
SU_OUT=$(su_war "$SU_SID" done "$SU_T1" 2>&1); SU_STATUS=$?
SU_REC=$(su_usage "$SU_T1")
SU_WANT='claude-x:10:5:100:20:135 claude-y:1:2:0:0:3 claude-z:3:4:0:0:7'
SU_GOT=$(python3 -c '
import json, sys
p = json.loads(sys.stdin.readline())
print(p["tokens"], p["source"], " ".join("%s:%s:%s:%s:%s:%s" % (m["model"], m["input"], m["output"], m["cache_read"], m["cache_write"], m["tokens"]) for m in p["models"]))
' <<<"$SU_REC" 2>&1)
SU_JSON=$(su_war "" --json status --usage 2>&1)
SU_TOTAL=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["tokens"])' <<<"$SU_JSON" 2>&1)
if [[ $SU_STATUS -eq 0 ]] && [[ "$SU_GOT" == "145 claude-code $SU_WANT" ]] && [[ "$SU_TOTAL" == 145 ]] \
    && [[ $(grep -c . <<<"$SU_REC") -eq 1 ]] && grep -q 'usage: 145 tok recorded' <<<"$SU_OUT"; then
    su_ok "a transcript gives exact totals" "145 = 135 + 3 + 7; the 9999 before the mark and a repeated line are not counted"
else
    su_fail "a transcript gives exact totals" "exit $SU_STATUS: got '$SU_GOT', status $SU_TOTAL"
fi

# The record holds numbers, never the transcript's words.
SU_SCAN=$(grep -rl 'FIXTURE-SECRET' "$SU_ROOT" 2>/dev/null; grep -l 'FIXTURE-SECRET' <<<"$SU_JSON$SU_OUT" 2>/dev/null)
if [[ -z "$SU_SCAN" ]] && grep -q 'FIXTURE-SECRET' "$SU_TX" && ! grep -rq "$SU_SID" "$SU_ROOT/docs"; then
    su_ok "no message content is recorded" "none in the repository, its git dir or the output; the grep finds it in the transcript"
else
    su_fail "no message content is recorded" "found in: $(tr '\n' ' ' <<<"$SU_SCAN")"
fi

# A session with no transcript: UNKNOWN and why, never 0.
SU_T2=$(su_new "Rotate keys")
su_war "nonexistent-session" start "$SU_T2" >/dev/null 2>&1
SU_OUT=$(su_war "nonexistent-session" done "$SU_T2" 2>&1)
SU_REC=$(su_usage "$SU_T2")
SU_HUMAN=$(su_war "" status --usage 2>&1)
if grep -q '"tokens": "unknown"' <<<"$SU_REC" && grep -q 'no Claude Code transcript' <<<"$SU_REC" \
    && ! grep -qE '"tokens": 0[,}]' <<<"$SU_REC" && grep -q 'usage: UNKNOWN' <<<"$SU_OUT" \
    && grep -q '(1 run(s) UNKNOWN)' <<<"$SU_HUMAN" && ! grep -qE "ticket $SU_T2 +0 tok" <<<"$SU_HUMAN"; then
    su_ok "a missing transcript is UNKNOWN" "journalled \"unknown\" with its reason; status counts it UNKNOWN, not 0"
else
    su_fail "a missing transcript is UNKNOWN" "$(head -c 300 <<<"$SU_REC") / $(head -1 <<<"$SU_HUMAN")"
fi

# No price table: no dollars anywhere. A dated one: an estimate, with its date.
SU_TODAY=$(date -u +%F)
SU_OLD=$(python3 -c 'import datetime as d; print((d.datetime.now(d.timezone.utc).date() - d.timedelta(days=400)).isoformat())')
if ! grep -q '\$' <<<"$SU_HUMAN" && grep -q '"dollars": null' <<<"$SU_JSON"; then
    cp "$SU_ROOT/openwarrant.toml" "$SU_TMP/openwarrant.toml.orig"
    printf '\n[usage]\nprices_as_of = "%s"\n\n[usage.prices."claude-x"]\ninput_per_mtok = 5.0\noutput_per_mtok = 25.0\ncache_read_per_mtok = 0.5\ncache_write_per_mtok = 6.25\n' \
        "$SU_TODAY" >>"$SU_ROOT/openwarrant.toml"
    SU_PRICED=$(su_war "" status --usage 2>&1)
    if grep -q "est., prices of $SU_TODAY" <<<"$SU_PRICED" && grep -q 'model claude-y  3 tok  no price' <<<"$SU_PRICED"; then
        su_ok "no price table, no dollars" "none without one; with a dated one, '~\$ est., prices of $SU_TODAY', and an unpriced model has none"
    else
        su_fail "no price table, no dollars" "with a table: $(head -3 <<<"$SU_PRICED" | tr '\n' ' ')"
    fi
else
    su_fail "no price table, no dollars" "dollars shown with no table: $(head -1 <<<"$SU_HUMAN")"
fi

# A stale table says so; a current one does not.
SU_RENDER=$(su_war "" tui --render usage 2>&1)
if ! grep -q 'STALE' <<<"$SU_PRICED$SU_RENDER" && grep -q "prices of $SU_TODAY" <<<"$SU_RENDER"; then
    sed -i "s/prices_as_of = \"$SU_TODAY\"/prices_as_of = \"$SU_OLD\"/" "$SU_ROOT/openwarrant.toml"
    SU_STALE=$(su_war "" status --usage 2>&1)
    SU_RENDER=$(su_war "" tui --render usage 2>&1)
    if grep -q "prices of $SU_OLD (STALE, 400 days old)" <<<"$SU_STALE" && grep -q "prices of $SU_OLD (STALE" <<<"$SU_RENDER"; then
        su_ok "a stale prices_as_of says STALE" "$SU_OLD: STALE, 400 days old, in status and Records; $SU_TODAY: not"
    else
        su_fail "a stale prices_as_of says STALE" "$(head -1 <<<"$SU_STALE")"
    fi
else
    su_fail "a stale prices_as_of says STALE" "a current table: $(grep -m1 -E 'TOTAL|STALE' <<<"$SU_RENDER")"
fi
cp "$SU_TMP/openwarrant.toml.orig" "$SU_ROOT/openwarrant.toml" 2>/dev/null

# Opted out: a session with a transcript records nothing, and no cursor.
SU_SID2=0f1c2d3e-0000-4000-8000-0000000a8e21
cp "$SU_FIX/after.jsonl" "$SU_CC/projects/-fixture/$SU_SID2.jsonl"
printf '\n[usage]\nrecord = false\n' >>"$SU_ROOT/openwarrant.toml"
SU_T3=$(su_new "Audit logs")
su_war "$SU_SID2" start "$SU_T3" >/dev/null 2>&1
cat "$SU_FIX/after.jsonl" >>"$SU_CC/projects/-fixture/$SU_SID2.jsonl"
SU_OUT=$(su_war "$SU_SID2" done "$SU_T3" 2>&1); SU_STATUS=$?
SU_CURSORS=$(find "$SU_ROOT/.git/openwarrant/usage-cursors" -name 'claude-code-*.json' 2>/dev/null | wc -l)
if [[ $SU_STATUS -eq 0 ]] && [[ -z "$(su_usage "$SU_T3")" ]] && ! grep -q 'usage:' <<<"$SU_OUT" \
    && [[ $SU_CURSORS -eq 1 ]]; then
    su_ok "an opt-out records nothing" "record = false: no work.usage, no second cursor; on, the first session recorded one"
else
    su_fail "an opt-out records nothing" "exit $SU_STATUS, cursors $SU_CURSORS: $(su_usage "$SU_T3" | head -c 200)"
fi
cp "$SU_TMP/openwarrant.toml.orig" "$SU_ROOT/openwarrant.toml" 2>/dev/null

# --usage: numbers only. Content is refused before anything is ticked.
SU_T4=$(su_new "Export data")
su_war "" start "$SU_T4" >/dev/null 2>&1
SU_OUT=$(printf '{"tokens":5,"content":"FIXTURE-SECRET"}' | su_war "" done "$SU_T4" --usage - 2>&1); SU_STATUS=$?
SU_OUT2=$(printf '{"model":"m","input":40,"output":2,"reporter":"opencode"}' | su_war "" done "$SU_T4" --usage - 2>&1); SU_STATUS2=$?
SU_REC=$(su_usage "$SU_T4")
if [[ $SU_STATUS -ne 0 ]] && grep -q 'usage.report-invalid' <<<"$SU_OUT" && [[ $(grep -c . <<<"$SU_REC") -eq 1 ]] \
    && [[ $SU_STATUS2 -eq 0 ]] && ! grep -q 'already done' <<<"$SU_OUT2" && grep -q '"source": "reported"' <<<"$SU_REC" && grep -q '"tokens": 42' <<<"$SU_REC" \
    && ! grep -q 'FIXTURE-SECRET' <<<"$SU_REC"; then
    su_ok "--usage takes numbers only" "content refused (usage.report-invalid), nothing ticked; numbers journalled as reported"
else
    su_fail "--usage takes numbers only" "exit $SU_STATUS/$SU_STATUS2: $(head -1 <<<"$SU_OUT") / $SU_REC"
fi

/usr/bin/rm -rf "$SU_TMP"
unset SU_TMP SU_WAR SU_FIX SU_ROOT SU_CC SU_SID SU_SID2 SU_TX SU_T1 SU_T2 SU_T3 SU_T4 SU_OUT SU_OUT2 SU_STATUS \
    SU_STATUS2 SU_REC SU_WANT SU_GOT SU_JSON SU_TOTAL SU_SCAN SU_HUMAN SU_TODAY SU_OLD SU_PRICED SU_RENDER \
    SU_STALE SU_CURSORS
