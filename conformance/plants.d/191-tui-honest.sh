# shellcheck shell=bash
# t-3c16c W5 — Mission Control shows what the records hold, and nothing else.
#
# The app cannot be driven from a pipe, so `war view tui --render <mode>` draws
# one frame as text, through the same read and the same draw a person gets.
# Each claim is paired with a refusal:
#
# - A repository with no recorded usage shows `TOKENS UNKNOWN`, no dollars, no
#   percentage. The old screen's fabricated figures are not in the source, and
#   the same grep is seen to catch one planted in a copy.
# - Every command line the app builds parses with the real CLI, and a planted
#   bad one is refused by name.
# - Without Herdr the Live panel is the working tree. With a stub daemon that
#   sends `null` where its schema allows it, the agents show, and an answer it
#   cannot read says UNKNOWN instead of a shorter fleet.
# - A real claim shows its holder and a renewal labelled as what it is.
# - Tokens a real `war evidence go` recorded are shown as recorded, a run that
#   reported none is UNKNOWN and not zero, and an undated price table is refused.

echo "== Mission Control is honest (t-3c16c W5) =="
TH_TMP=$(mktemp -d)
TH_WAR="$REPO_ROOT/${WAR#./}"
th_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
th_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
TH_ROOT="$TH_TMP/repo"
mkdir -p "$TH_ROOT"
git -C "$TH_ROOT" init -q .
th_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID HERDR_SOCKET_PATH="$TH_TMP/no.sock" OPENWARRANT_ACTOR=agent "$TH_WAR" --root "$TH_ROOT" "$@"; }
th_war init >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: war init\n' >&2; exit 9; }
TH_TICKET=$(th_war --json create "Reset passwords" --item "Email template" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])')
[[ -n "$TH_TICKET" ]] || { printf 'PLANT SETUP FAILED: no ticket\n' >&2; exit 9; }

# No usage recorded: UNKNOWN, and nothing that looks like a measurement.
TH_WATCH=$(th_war tui --render watch 2>&1); TH_STATUS=$?
if [[ $TH_STATUS -eq 0 ]] && grep -q 'TOKENS UNKNOWN' <<<"$TH_WATCH" && grep -q 'Reset passwords' <<<"$TH_WATCH" \
    && ! grep -qE '\$[0-9]|[0-9]+% complete|k tok' <<<"$TH_WATCH"; then
    th_ok "no usage reads UNKNOWN" "TOKENS UNKNOWN, no dollars, no percentage"
else
    th_fail "no usage reads UNKNOWN" "exit $TH_STATUS: $(head -5 <<<"$TH_WATCH" | tr '\n' ' ')"
fi

# Progress is counts from the records.
if grep -qE 'ITEMS 1 open, 0 active, 0 done' <<<"$TH_WATCH"; then
    th_ok "progress is counts" "ITEMS 1 open, 0 active, 0 done"
else
    th_fail "progress is counts" "$(grep -m1 ITEMS <<<"$TH_WATCH")"
fi

# The figures the old screen made up are gone, and the grep can see one.
TH_FAKE='28_400|31_200|48_500|0\.000015|LEASES: fresh|% complete|DISPATCH_MODELS'
th_fabricated() { grep -rnE "$TH_FAKE" "$1" --include='*.rs' --exclude=tests.rs; }
if ! th_fabricated crates/openwarrant-cli/src/tui >/dev/null; then
    mkdir "$TH_TMP/planted"
    printf 'const BURN: u64 = 28_400;\n' >"$TH_TMP/planted/x.rs"
    if th_fabricated "$TH_TMP/planted" >/dev/null; then
        th_ok "no fabricated constants" "none in tui/, and a planted one is seen"
    else
        th_fail "no fabricated constants" "the grep did not see the planted constant"
    fi
else
    th_fail "no fabricated constants" "$(th_fabricated crates/openwarrant-cli/src/tui | head -3 | tr '\n' ' ')"
fi

# Every argv parses; a planted bad one fails, by name.
TH_OUT=$(th_war tui --check-argv 2>&1); TH_STATUS=$?
if [[ $TH_STATUS -eq 0 ]] && grep -q 'command line(s) parse' <<<"$TH_OUT"; then
    th_ok "every argv parses" "$TH_OUT"
else
    th_fail "every argv parses" "exit $TH_STATUS: $(head -3 <<<"$TH_OUT" | tr '\n' ' ')"
fi
printf 'view show t-1\nticket new oops\n' >"$TH_TMP/bad-argv"
TH_OUT=$(th_war tui --check-argv "$TH_TMP/bad-argv" 2>&1); TH_STATUS=$?
if [[ $TH_STATUS -eq 2 ]] && grep -q 'tui.bad-argv' <<<"$TH_OUT" && grep -q 'ticket new oops' <<<"$TH_OUT" \
    && ! grep -q 'view show t-1' <<<"$TH_OUT"; then
    th_ok "a planted bad argv fails" "exit 2, tui.bad-argv, names only the bad line"
else
    th_fail "a planted bad argv fails" "exit $TH_STATUS: $(head -3 <<<"$TH_OUT" | tr '\n' ' ')"
fi

# Without Herdr: the working tree, and it says so.
printf 'x\n' >"$TH_ROOT/untracked-file"
TH_WATCH=$(th_war tui --render watch 2>&1)
if grep -q 'herdr: not running' <<<"$TH_WATCH" && grep -q 'working tree and journal' <<<"$TH_WATCH" \
    && grep -q 'untracked-file' <<<"$TH_WATCH"; then
    th_ok "without Herdr, the working tree" "real git status in the Live panel"
else
    th_fail "without Herdr, the working tree" "$(grep -m1 -n 'herdr' <<<"$TH_WATCH")"
fi

# With Herdr: a stub that answers as the real one does, nulls included.
th_herdr() {  # <socket> <agents JSON>
    python3 - "$1" "$2" <<'PY' &
import json, os, socket, sys
path, agents = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX)
s.bind(path)
s.listen(8)
s.settimeout(30)
while True:
    try:
        c, _ = s.accept()
    except OSError:
        break
    req = json.loads(c.makefile().readline())
    m = req["method"]
    if m == "ping":
        r = {"type": "pong", "version": "9.9.9", "protocol": 22}
    elif m == "agent.list":
        r = {"type": "agent_list", "agents": json.loads(agents)}
    else:
        r = {"type": "pane_read", "read": {"text": "STUB TERMINAL TEXT"}}
    c.sendall((json.dumps({"id": req["id"], "result": r}) + "\n").encode())
    c.close()
PY
    TH_STUB=$!
    for _ in 1 2 3 4 5 6 7 8 9 10; do [[ -S "$1" ]] && break; sleep 0.2; done
}
th_war_herdr() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID HERDR_SOCKET_PATH="$1" OPENWARRANT_ACTOR=agent "$TH_WAR" --root "$TH_ROOT" tui --render watch; }
th_herdr "$TH_TMP/h.sock" '[{"pane_id":"w1:p1","terminal_id":"t","agent":null,"name":null,"agent_status":"unknown","cwd":null,"foreground_cwd":null,"terminal_title":null,"terminal_title_stripped":null,"workspace_id":"w1","tab_id":"w1:t1","focused":false,"revision":1}]'
TH_WATCH=$(th_war_herdr "$TH_TMP/h.sock" 2>&1)
kill "$TH_STUB" 2>/dev/null; wait "$TH_STUB" 2>/dev/null
if grep -q 'herdr: 9.9.9, 0 here of 1 agent' <<<"$TH_WATCH" && grep -q 'STUB TERMINAL TEXT' <<<"$TH_WATCH"; then
    th_ok "a null from Herdr is read" "the agent is listed, its terminal shown"
else
    th_fail "a null from Herdr is read" "$(grep -m1 -n 'herdr' <<<"$TH_WATCH")"
fi
th_herdr "$TH_TMP/h2.sock" '[{"pane_id":7}]'
TH_WATCH=$(th_war_herdr "$TH_TMP/h2.sock" 2>&1)
kill "$TH_STUB" 2>/dev/null; wait "$TH_STUB" 2>/dev/null
if grep -q 'UNKNOWN: Herdr answered' <<<"$TH_WATCH"; then
    th_ok "an unreadable fleet is UNKNOWN" "not a shorter list"
else
    th_fail "an unreadable fleet is UNKNOWN" "$(grep -m1 -n 'herdr' <<<"$TH_WATCH")"
fi

# A real claim: holder, lease, and the renewal under its own name.
th_war admin claim "$TH_TICKET" --paths 'src/**' >/dev/null 2>&1
TH_WATCH=$(th_war tui --render watch 2>&1)
if grep -q 'CLAIMS 1 (1 live)' <<<"$TH_WATCH" && grep -q 'lock renewed' <<<"$TH_WATCH" \
    && ! grep -qi 'heartbeat' <<<"$TH_WATCH" && grep -q "agent $TH_TICKET" <<<"$TH_WATCH"; then
    th_ok "a claim says what it knows" "holder, lease, lock renewed"
else
    th_fail "a claim says what it knows" "$(grep -m2 -E 'CLAIMS|lock' <<<"$TH_WATCH" | tr '\n' ' ')"
fi

# Recorded usage: a real `war evidence go` whose harness reports its tokens, and
# one whose harness reports none. The total is that number, the second run is
# counted as UNKNOWN, not as zero, and nothing prices either: go.ended holds no
# model, and without a dated table there are no dollars.
cat >"$TH_TMP/reports.py" <<'PY'
import json, os, subprocess, sys
fixture = os.environ["TH_FIXTURE"]
out = subprocess.run([sys.executable, fixture], stdin=sys.stdin, capture_output=True, text=True).stdout
if os.environ.get("TH_REPORT"):
    answer = json.loads(out)
    answer["usage"] = {"tokens": 1234}
    out = json.dumps(answer)
sys.stdout.write(out)
PY
cat >>"$TH_ROOT/openwarrant.toml" <<EOF

[go]
harness_argv = ["python3", "$TH_TMP/reports.py"]
EOF
th_go() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR HERDR_SOCKET_PATH="$TH_TMP/no.sock" \
    TH_FIXTURE="$REPO_ROOT/conformance/fixtures/go/performer.py" GO_LOG="$TH_TMP/go.log" "$TH_WAR" --root "$TH_ROOT" "$@" </dev/null; }
th_war admin release "$TH_TICKET" >/dev/null 2>&1
TH_ONE=$(th_war --json create "Reports its tokens" --item "one" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])')
git -C "$TH_ROOT" add -A >/dev/null 2>&1
git -C "$TH_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm tickets >/dev/null 2>&1
TH_REPORT=1 th_go evidence go --max-nodes 1 >/dev/null 2>&1
th_go evidence go --max-nodes 1 >/dev/null 2>&1
TH_WATCH=$(th_war tui --render watch 2>&1)
if grep -q 'TOKENS 1234 (1 run(s) UNKNOWN)' <<<"$TH_WATCH" && ! grep -qE '~\$[0-9]' <<<"$TH_WATCH"; then
    th_ok "recorded usage is shown as recorded" "1234, one run UNKNOWN, no dollars"
else
    th_fail "recorded usage is shown as recorded" "$(grep -m1 TOKENS <<<"$TH_WATCH"); ${TH_ONE:-no ticket}"
fi
# Refusal: an undated price table shows no dollars, and says why.
cat >>"$TH_ROOT/openwarrant.toml" <<'EOF'

[usage.prices.m]
input_per_mtok = 2
output_per_mtok = 10
EOF
TH_WATCH=$(th_war tui --render watch 2>&1)
if grep -q 'prices_as_of' <<<"$TH_WATCH" && ! grep -qE '~\$[0-9]' <<<"$TH_WATCH"; then
    th_ok "an undated price table is refused" "said on screen, no dollars"
else
    th_fail "an undated price table is refused" "$(grep -m1 TOKENS <<<"$TH_WATCH")"
fi

/usr/bin/rm -rf "$TH_TMP"
unset TH_ONE TH_TMP TH_WAR TH_ROOT TH_TICKET TH_WATCH TH_OUT TH_STATUS TH_STUB TH_FAKE
