# shellcheck shell=bash
# OW-WAR-0112 (absorbing OW-WAR-0073) — `war` is the app, and holds nothing.
#
# Read-only on this corpus. The app cannot be driven from a pipe — that is
# the first plant — so these hold the invariants a shell can see: no key or
# socket under tui/, a refusal by name without a terminal, the queue equal
# to `war sign --list`, and the terminal restored after a panic.

echo "== the app (OW-WAR-0112) =="
TUI_SRC=crates/openwarrant-cli/src/tui

# OBL-002: no key, no socket, no signature construction under tui/. Comment
# lines are allowed to NAME the things they forbid; code lines are not.
if ! grep -rn 'ssh-keygen\|SSH_AUTH_SOCK\|ssh_sign(\|sign::run(\|sign::ingest\|authorize::ingest\|resolution_cmd::ingest\|correct::ingest' "$TUI_SRC" \
    | grep -v ':\s*//' | grep -q .; then
    printf 'ok    %-34s no key, no socket, no signing call under tui/\n' "the app holds no authority"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s tui/ names a key, a socket or a signing seam\n' "the app holds no authority"
    FAILED=$((FAILED + 1))
fi

# Every act shells out: the child is `war sign … --ssh-sign`, spelled out.
if grep -q '"sign", target, "--ssh-sign"' "$TUI_SRC/mod.rs"; then
    printf 'ok    %-34s every act is a child `war sign --ssh-sign`\n' "acts shell out"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the signing child is not spelled out\n' "acts shell out"
    FAILED=$((FAILED + 1))
fi

# Without a terminal: `war` alone exits 2 by name and writes no escape codes.
TUI_OUT=$("$WAR" </dev/null 2>&1 | cat)
TUI_STATUS=${PIPESTATUS[0]}
TUI_OUT2=$("$WAR" </dev/null 2>&1; echo "exit=$?")
if grep -q 'exit=2' <<<"$TUI_OUT2" && grep -q 'tui.no-tty' <<<"$TUI_OUT2" \
    && ! grep -q $'\x1b\[?1049h' <<<"$TUI_OUT2" && grep -q 'war status --json' <<<"$TUI_OUT2"; then
    printf 'ok    %-34s exit 2, tui.no-tty, names war status --json\n' "no terminal, no app"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "no terminal, no app" "$(tail -1 <<<"$TUI_OUT2")"
    FAILED=$((FAILED + 1))
fi
: "$TUI_OUT" "$TUI_STATUS"

# `war tui` by name behaves the same; `war --json` alone names the envelopes.
TUI_OUT3=$("$WAR" tui </dev/null 2>&1; echo "exit=$?")
TUI_OUT4=$("$WAR" --json </dev/null 2>&1; echo "exit=$?")
if grep -q 'exit=2' <<<"$TUI_OUT3" && grep -q 'tui.no-tty' <<<"$TUI_OUT3" \
    && grep -q 'exit=1' <<<"$TUI_OUT4" && grep -q 'war console --json' <<<"$TUI_OUT4"; then
    printf 'ok    %-34s war tui refuses alike; war --json names the envelopes\n' "tui by name and --json"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s tui: %s / json: %s\n' "tui by name and --json" "$(tail -1 <<<"$TUI_OUT3")" "$(tail -1 <<<"$TUI_OUT4")"
    FAILED=$((FAILED + 1))
fi

# OBL-003: the queue's rows are `war sign --list`'s — the board the app
# renders is the console's board, compared through its JSON.
TUI_LIST=$("$WAR" sign --list 2>&1)
if "$WAR" console --json 2>/dev/null | python3 -c '
import sys, json
d = json.load(sys.stdin)
acts = [a["target"] for a in d["result"]["acts"]]
listed = sys.argv[1]
assert acts, "no act on the board to compare"
for t in acts:
    assert t in listed, f"{t} on the board, not in war sign --list"
' "$TUI_LIST" 2>/dev/null; then
    printf 'ok    %-34s board acts == sign --list rows\n' "the queue is war sign --list"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the board and sign --list disagree\n' "the queue is war sign --list"
    FAILED=$((FAILED + 1))
fi

# OBL-002: a panic after setup leaves the terminal restored — the alternate
# screen is left (ESC[?1049l) after it was entered, under a pty.
if command -v script >/dev/null 2>&1; then
    TUI_PTY=$(script -qec "$WAR tui --panic-after-setup" /dev/null 2>&1 | cat -v)
    if grep -q '\^\[\[?1049h' <<<"$TUI_PTY" && grep -q '\^\[\[?1049l' <<<"$TUI_PTY" && grep -q 'panic-after-setup' <<<"$TUI_PTY"; then
        printf 'ok    %-34s alternate screen entered and left, panic printed\n' "a panic restores the terminal"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "a panic restores the terminal" "$(tr -d '\r' <<<"$TUI_PTY" | tail -c 200 | tr '\n' ' ')"
        FAILED=$((FAILED + 1))
    fi
else
    printf 'ok    %-34s (script(1) absent; the pty plant did not run)\n' "a panic restores the terminal"
    PASSED=$((PASSED + 1))
fi

# The keys the app renders on `?` are the keys docs/TUI.md documents: a
# unit test holds it (tui::tests); the plant holds that the document exists
# and names every pane.
if [[ -f docs/TUI.md ]] && ( for p in Setup Help Queue Questions Frontier Corpus Obligations Evidence Journal; do grep -q "^## $p" docs/TUI.md || exit 1; done ); then
    printf 'ok    %-34s nine panes documented\n' "docs/TUI.md names every pane"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s docs/TUI.md missing or a pane undocumented\n' "docs/TUI.md names every pane"
    FAILED=$((FAILED + 1))
fi
