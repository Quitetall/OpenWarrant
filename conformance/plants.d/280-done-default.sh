# shellcheck shell=bash
# alpha.8 R1: `war done` checks by default (`[tickets] done_test`). Each claim
# beside its refusal:
#   init records the test it finds and says so; with none to find it writes
#   no key and says nothing;
#   a passing default ticks at observed; a failing one refuses and ticks
#   nothing; one that cannot finish is UNKNOWN, never a pass or a fail;
#   a ticket's own test decides and the default is not run;
#   no key is the claimed tick, in the words it always had;
#   `--done-test` sets and replaces the key in a repository already set up,
#   twice is the same bytes, and with nothing to find it is refused;
#   it runs once per `war done`; an unchanged tree reuses the pass and a
#   changed one runs it again;
#   an Order still waits for its signer after the default passes, and is not
#   handed over when it fails;
#   MCP `war_done` is the same code.

echo "== war done checks by default (alpha.8 R1) =="
for v in OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID \
         CLAUDE_CODE_SESSION_ID OPENWARRANT_ACTOR; do unset "$v"; done
DT_T=$(mktemp -d)
dt_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
dt_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
DT_WAR="$REPO_ROOT/${WAR#./}"
dtw() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE OPENWARRANT_NO_PROJECTS=1 OPENWARRANT_ACTOR=plant-agent \
    "$DT_WAR" --root "$DT_R" "$@" </dev/null; }
dt_id() { python3 -c 'import json,sys; v=json.load(sys.stdin)["result"]; print(v["id"], *[i["id"] for i in v.get("items", [])])'; }
dt_runs() { if [[ -f "$DT_T/count" ]]; then wc -l < "$DT_T/count" | tr -d ' '; else echo 0; fi; }
dt_doc() { cat "$(tk "$DT_R/docs/tickets/$1.md")" 2>/dev/null; }
dt_new() { # a git repository holding a counting test that passes when `ok` exists
    DT_R="$DT_T/$1"; mkdir -p "$DT_R"; git -C "$DT_R" init -q .
    printf 'echo run >> "%s"\ntest -f ok\n' "$DT_T/count" > "$DT_R/check.sh"
    : > "$DT_T/count"
}

# ---- init records what it finds ------------------------------------------------
dt_new found; printf 'test:\n\tsh check.sh\n' > "$DT_R/Makefile"
DT_OUT=$(dtw init --namespace DT 2>&1); DT_S=$?
if [[ $DT_S -eq 0 ]] && grep -q '^done_test = "make test"$' "$DT_R/openwarrant.toml" \
    && grep -qF '[tickets] done_test = "make test"' <<<"$DT_OUT" && grep -qF 'delete the line to turn it off' <<<"$DT_OUT"; then
    dt_ok "init records the detected test" 'done_test = "make test", said in one line'
else
    dt_fail "init records the detected test" "exit $DT_S: $(tail -2 <<<"$DT_OUT" | tr '\n' '|')"
fi
DT_FOUND=$DT_R
dt_new none
DT_OUT=$(dtw init --namespace DN 2>&1); DT_S=$?
if [[ $DT_S -eq 0 ]] && ! grep -q 'done_test\|^\[tickets\]' "$DT_R/openwarrant.toml" && ! grep -q 'done_test' <<<"$DT_OUT"; then
    dt_ok "nothing found, nothing recorded" "no key and no line"
else
    dt_fail "nothing found, nothing recorded" "exit $DT_S: $(grep done_test "$DT_R/openwarrant.toml" <<<"$DT_OUT")"
fi
DT_NONE=$DT_R

# ---- no key: the claimed tick ---------------------------------------------------
read -r DT_ID _ <<<"$(dtw --json create "Plain" | dt_id)"
DT_OUT=$(dtw done "$DT_ID" 2>&1); DT_S=$?
if [[ $DT_S -eq 0 ]] && grep -qF 'ticked as claimed: nothing was checked' <<<"$DT_OUT" \
    && ! grep -q observed <<<"$(dt_doc "$DT_ID")" && [[ $(dt_runs) == 0 ]]; then
    dt_ok "no key ticks as claimed" "nothing ran, and it says so"
else
    dt_fail "no key ticks as claimed" "exit $DT_S: $(tr '\n' '|' <<<"$DT_OUT")"
fi

# ---- --done-test on a repository already set up -----------------------------------
DT_BEFORE=$(cat "$DT_R/openwarrant.toml")
DT_OUT=$(dtw init --done-test 2>&1); DT_S=$?
if [[ $DT_S -ne 0 && "$(cat "$DT_R/openwarrant.toml")" == "$DT_BEFORE" ]] && grep -qF 'no test command was found' <<<"$DT_OUT"; then
    dt_ok "--done-test with nothing to find" "refused, exit $DT_S, the file untouched"
else
    dt_fail "--done-test with nothing to find" "exit $DT_S: $DT_OUT"
fi
dtw init --done-test "false" >/dev/null 2>&1; DT_S=$?
DT_OUT=$(dtw init --done-test "sh check.sh" 2>&1); DT_S2=$?
DT_ONCE=$(cksum < "$DT_R/openwarrant.toml")
dtw init --done-test "sh check.sh" >/dev/null 2>&1; DT_S3=$?
if [[ $DT_S -eq 0 && $DT_S2 -eq 0 && $DT_S3 -eq 0 && "$(cksum < "$DT_R/openwarrant.toml")" == "$DT_ONCE" \
    && $(grep -c '^done_test = ' "$DT_R/openwarrant.toml") == 1 \
    && "$(grep -v '^done_test = \|^\[tickets\]$' "$DT_R/openwarrant.toml" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')" == "$DT_BEFORE" ]] \
    && grep -q '^done_test = "sh check.sh"$' "$DT_R/openwarrant.toml"; then
    dt_ok "--done-test sets and replaces" "one key, the rest of the file as it was, twice the same bytes"
else
    dt_fail "--done-test sets and replaces" "exit $DT_S/$DT_S2/$DT_S3: $(grep -n 'done_test\|tickets' "$DT_R/openwarrant.toml" | tr '\n' '|') $DT_OUT"
fi

# ---- a failing default refuses; a passing one ticks at observed --------------------
read -r DT_ID _ <<<"$(dtw --json create "Checked" | dt_id)"
dtw start "$DT_ID" >/dev/null 2>&1
DT_OUT=$(dtw done "$DT_ID" 2>&1); DT_S=$?
DT_J="$(tk "$DT_R/docs/tickets/$DT_ID/journal.jsonl")"
if [[ $DT_S -eq 2 ]] && grep -qF 'refused (ticket.check-failed)' <<<"$DT_OUT" && grep -qF 'Nothing was ticked' <<<"$DT_OUT" \
    && ! grep -q 'work.item_done' "$DT_J" && grep -q '"type":"work.check_run"' "$DT_J" \
    && [[ "$(dtw --json view show "$DT_ID" | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["state"])')" != done ]]; then
    dt_ok "a failing default ticks nothing" "ticket.check-failed, exit 2, the run journalled"
else
    dt_fail "a failing default ticks nothing" "exit $DT_S: $(tr '\n' '|' <<<"$DT_OUT" | cut -c1-300)"
fi
: > "$DT_R/ok"
DT_OUT=$(dtw done "$DT_ID" 2>&1); DT_S=$?
DT_CHECK=$(dtw check 2>&1)
if [[ $DT_S -eq 0 ]] && grep -qxF 'ticked at observed: default passed' <<<"$DT_OUT" \
    && grep -q '"level":"observed"' "$DT_J" && grep -q '"default":{"cmd":"sh check.sh"' "$DT_J" \
    && ! grep -q 'tick-unbacked' <<<"$DT_CHECK"; then
    dt_ok "a passing default ticks at observed" "said, journalled with its receipt, and backed"
else
    dt_fail "a passing default ticks at observed" "exit $DT_S: $(tr '\n' '|' <<<"$DT_OUT" | cut -c1-300)"
fi

# ---- UNKNOWN is neither -----------------------------------------------------------
printf 'done_test_timeout = 1\n' >> "$DT_R/openwarrant.toml"
sed -i 's/^done_test = .*/done_test = "sleep 20"/' "$DT_R/openwarrant.toml"
read -r DT_ID _ <<<"$(dtw --json create "Cannot finish" | dt_id)"
dtw start "$DT_ID" >/dev/null 2>&1
DT_OUT=$(dtw done "$DT_ID" 2>&1); DT_S=$?
if [[ $DT_S -ne 0 ]] && grep -qF 'UNKNOWN (ticket.check-unknown)' <<<"$DT_OUT" \
    && ! grep -q 'check-failed\|ticked at' <<<"$DT_OUT" && ! grep -q 'work.item_done' "$(tk "$DT_R/docs/tickets/$DT_ID/journal.jsonl")"; then
    dt_ok "a default that cannot finish" "UNKNOWN (ticket.check-unknown), exit $DT_S: not a pass, not a fail, nothing ticked"
else
    dt_fail "a default that cannot finish" "exit $DT_S: $(tr '\n' '|' <<<"$DT_OUT" | cut -c1-300)"
fi
dtw admin release "$DT_ID" >/dev/null 2>&1
sed -i -e '/^done_test_timeout/d' -e 's/^done_test = .*/done_test = "sh check.sh"/' "$DT_R/openwarrant.toml"

# ---- the ticket's own test takes precedence -----------------------------------------
read -r DT_ID _ <<<"$(dtw --json create "Own test" --test "test -f own" | dt_id)"
dtw start "$DT_ID" >/dev/null 2>&1
DT_N=$(dt_runs)
DT_OUT=$(dtw done "$DT_ID" 2>&1); DT_S=$?
: > "$DT_R/own"
DT_OUT2=$(dtw done "$DT_ID" 2>&1); DT_S2=$?
if [[ $DT_S -eq 2 ]] && grep -qF 'check.failed' <<<"$DT_OUT" && [[ $DT_S2 -eq 0 && $(dt_runs) == "$DT_N" ]] \
    && ! grep -q 'default' <<<"$DT_OUT$DT_OUT2" && ! grep -q '"default"' "$(tk "$DT_R/docs/tickets/$DT_ID/journal.jsonl")"; then
    dt_ok "a ticket's own test decides" "the passing default neither stood in for it nor ran"
else
    dt_fail "a ticket's own test decides" "exit $DT_S then $DT_S2, runs $DT_N -> $(dt_runs): $(tr '\n' '|' <<<"$DT_OUT2" | cut -c1-200)"
fi

# ---- once per call; reused only while nothing changed -------------------------------
read -r DT_ID DT_A DT_B DT_C <<<"$(dtw --json create "Three" -i a -i b -i c | dt_id)"
read -r DT_ID2 _ <<<"$(dtw --json create "Two at once" -i a -i b | dt_id)"
dtw start "$DT_ID2" >/dev/null 2>&1
DT_N=$(dt_runs)
DT_OUT=$(dtw done "$DT_ID2" 2>&1); DT_S=$?
if [[ $DT_S -eq 0 && $(dt_runs) == $((DT_N + 1)) && $(dt_doc "$DT_ID2" | grep -c '\[observed\]') == 2 ]]; then
    dt_ok "it runs once per war done" "two items, one run, both observed"
else
    dt_fail "it runs once per war done" "exit $DT_S, runs $DT_N -> $(dt_runs): $(tr '\n' '|' <<<"$DT_OUT" | cut -c1-200)"
fi
dtw start "$DT_ID" >/dev/null 2>&1
dtw done "$DT_ID/$DT_A" >/dev/null 2>&1
DT_N=$(dt_runs)
DT_OUT=$(dtw done "$DT_ID/$DT_B" 2>&1); DT_S=$?
DT_N2=$(dt_runs)
printf 'changed\n' > "$DT_R/source.txt"
DT_OUT2=$(dtw done "$DT_ID/$DT_C" 2>&1); DT_S2=$?
if [[ $DT_S -eq 0 && $DT_N2 == "$DT_N" ]] && grep -qF 'default passed (not rerun: nothing changed since it passed)' <<<"$DT_OUT" \
    && [[ $DT_S2 -eq 0 && $(dt_runs) == $((DT_N + 1)) ]] && ! grep -q 'not rerun' <<<"$DT_OUT2"; then
    dt_ok "a pass is reused only unchanged" "the same bytes: not rerun, and said; one new file: run again"
else
    dt_fail "a pass is reused only unchanged" "exit $DT_S/$DT_S2, runs $DT_N/$DT_N2/$(dt_runs): $(tr '\n' '|' <<<"$DT_OUT" | cut -c1-200)"
fi

# ---- an Order still waits for its signer --------------------------------------------
read -r DT_ID DT_A DT_B <<<"$(dtw --json create "An Order" -i "the release" -i "the notes" | dt_id)"
dtw plan add "$DT_ID/$DT_A" --min signed >/dev/null 2>&1
dtw start "$DT_ID" >/dev/null 2>&1
command rm -f "$DT_R/ok"
DT_OUT=$(dtw done "$DT_ID" 2>&1); DT_S=$?
DT_J="$(tk "$DT_R/docs/tickets/$DT_ID/journal.jsonl")"
DT_HANDED=$(grep -c 'work.awaits_signature' "$DT_J")
: > "$DT_R/ok"
DT_OUT2=$(dtw done "$DT_ID" 2>&1); DT_S2=$?
if [[ $DT_S -eq 2 && $DT_HANDED == 0 ]] && grep -qF 'ticket.check-failed' <<<"$DT_OUT" \
    && [[ $DT_S2 -eq 0 ]] && grep -qF "$DT_ID waits on a person: \`war sign $DT_ID/$DT_A --ssh-sign\`" <<<"$DT_OUT2" \
    && grep -qF 'ticked at observed: default passed' <<<"$DT_OUT2" && grep -q 'work.awaits_signature' "$DT_J" \
    && grep -q '^- \[ \] the release' "$(tk "$DT_R/docs/tickets/$DT_ID.md")"; then
    dt_ok "an Order still waits for its signer" "failing: not handed over; passing: the agent's item observed, the signature item open"
else
    dt_fail "an Order still waits for its signer" "exit $DT_S ($DT_HANDED handed) then $DT_S2: $(tr '\n' '|' <<<"$DT_OUT2" | cut -c1-300)"
fi

# ---- MCP war_done is the same code ----------------------------------------------------
read -r DT_ID _ <<<"$(dtw --json create "Over MCP" | dt_id)"
{
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"plant","version":"1"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"war_start","arguments":{"target":"%s","actor":"mcp-agent"}}}\n' "$DT_ID"
    printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"war_done","arguments":{"target":"%s","actor":"mcp-agent"}}}\n' "$DT_ID"
} > "$DT_T/tx"
command rm -f "$DT_R/ok"
DT_MCP=$(cd "$DT_R" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_NO_PROJECTS=1 timeout 60 python3 "$REPO_ROOT/conformance/fixtures/mcp/drive.py" "$DT_WAR" "$DT_T/tx" 2>/dev/null)
DT_OPEN=$(grep -c 'work.item_done' "$(tk "$DT_R/docs/tickets/$DT_ID/journal.jsonl")")
: > "$DT_R/ok"
sed -i '/war_start/d' "$DT_T/tx"
DT_MCP2=$(cd "$DT_R" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_NO_PROJECTS=1 timeout 60 python3 "$REPO_ROOT/conformance/fixtures/mcp/drive.py" "$DT_WAR" "$DT_T/tx" 2>/dev/null)
if grep -qF 'ticket.check-failed' <<<"$DT_MCP" && [[ $DT_OPEN == 0 ]] && grep -qF 'ticked at observed: default passed' <<<"$DT_MCP2"; then
    dt_ok "war_done over MCP is the same" "refused while it fails, observed once it passes"
else
    dt_fail "war_done over MCP is the same" "$(cut -c1-300 <<<"$DT_MCP2")"
fi

command rm -rf "$DT_T"
unset DT_CHECK DT_T DT_WAR DT_R DT_OUT DT_OUT2 DT_S DT_S2 DT_S3 DT_FOUND DT_NONE DT_ID DT_ID2 DT_A DT_B DT_C DT_J DT_N DT_N2 \
    DT_BEFORE DT_ONCE DT_HANDED DT_MCP DT_MCP2 DT_OPEN
unset -f dt_ok dt_fail dtw dt_id dt_runs dt_doc dt_new
