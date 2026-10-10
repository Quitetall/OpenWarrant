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
if ! { TUI_HITS=$(grep -rn 'ssh-keygen\|SSH_AUTH_SOCK\|ssh_sign(\|sign::run(\|sign::ingest\|authorize::ingest\|resolution_cmd::ingest\|correct::ingest' "$TUI_SRC" \
    | grep -v ':\s*//') && grep -q . <<<"$TUI_HITS"; }; then
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

# Without a terminal: `war` alone exits 2 by name, then clap's usage, and
# writes no escape code at all — not the alternate screen, not a colour.
TUI_OUT2=$("$WAR" </dev/null 2>&1; echo "exit=$?")
if grep -q 'exit=2' <<<"$TUI_OUT2" && grep -q 'tui.no-tty' <<<"$TUI_OUT2" \
    && grep -q '^Usage: war \[OPTIONS\] \[COMMAND\]' <<<"$TUI_OUT2" \
    && ! grep -q $'\x1b' <<<"$TUI_OUT2" && grep -q 'war status --json' <<<"$TUI_OUT2"; then
    printf 'ok    %-34s exit 2, tui.no-tty, usage, names war status --json, no escape\n' "no terminal, no app"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "no terminal, no app" "$(tail -3 <<<"$TUI_OUT2" | tr '\n' '|')"
    FAILED=$((FAILED + 1))
fi

# `war tui` by name behaves the same.
TUI_OUT3=$("$WAR" tui </dev/null 2>&1; echo "exit=$?")
if grep -q 'exit=2' <<<"$TUI_OUT3" && grep -q 'tui.no-tty' <<<"$TUI_OUT3"; then
    printf 'ok    %-34s exit 2, tui.no-tty\n' "war tui from a pipe"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "war tui from a pipe" "$(tail -1 <<<"$TUI_OUT3")"
    FAILED=$((FAILED + 1))
fi

# `war --json` alone, and `war tui --json`: a rendering has no envelope, so
# each is refused by name (`tui.json`) IN an envelope, exit 2, naming
# `war view console --json` and `war status --json`. The envelope's own exit_code
# must be the process's.
for TUI_ARGS in "--json" "tui --json"; do
    # shellcheck disable=SC2086 # the two words are two arguments
    TUI_JSON=$("$WAR" $TUI_ARGS </dev/null 2>/dev/null); TUI_STATUS=$?
    if [[ $TUI_STATUS -eq 2 ]] && python3 -c '
import sys, json
d = json.load(sys.stdin)
assert d["schema"] == "oh.war/report/v1" and d["exit_code"] == 2, d
[x] = [x for x in d["diagnostics"] if x["severity"] == "error"]
assert x["rule"] == "tui.json", x
assert "`war view console --json`" in x["message"] and "`war status --json`" in x["message"], x
' <<<"$TUI_JSON" 2>/dev/null; then
        printf 'ok    %-34s exit 2, tui.json, names war view console/status --json\n' "war $TUI_ARGS refuses by name"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s exit %s: %s\n' "war $TUI_ARGS refuses by name" "$TUI_STATUS" "$(head -c 200 <<<"$TUI_JSON" | tr '\n' ' ')"
        FAILED=$((FAILED + 1))
    fi
done

# The `sdk` argv scan is untouched: `war --json sdk --request -` still answers
# with an envelope — an ill-formed request refused inside one, never clap's
# usage and never the app.
TUI_SDK=$(printf '{}' | "$WAR" --json sdk --request - 2>/dev/null); TUI_STATUS=$?
if python3 -c '
import sys, json
d = json.load(sys.stdin)
assert d["schema"] == "oh.war/report/v1", d
assert not any(x["rule"].startswith("tui.") for x in d["diagnostics"]), d
' <<<"$TUI_SDK" 2>/dev/null && ! grep -q '^Usage:' <<<"$TUI_SDK"; then
    printf 'ok    %-34s an envelope (exit %s), not usage, not the app\n' "war --json sdk --request -" "$TUI_STATUS"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s: %s\n' "war --json sdk --request -" "$TUI_STATUS" "$(head -c 200 <<<"$TUI_SDK" | tr '\n' ' ')"
    FAILED=$((FAILED + 1))
fi

# OBL-003: the queue's rows are `war sign --list`'s — the board the app
# renders is the console's board, compared through its JSON. Over a scratch
# program whose queue holds two authorizations and a SAS acceptance (lib.sh,
# `scratch_queue`): this corpus's queue is empty whenever the owner has
# signed everything, and an empty board has nothing to compare.
TUI_ROOT=$(scratch_queue TQ)
[[ -d "${TUI_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
TUI_LIST=$("$WAR" --root "$TUI_ROOT" sign --list 2>&1)
if "$WAR" --root "$TUI_ROOT" console --json 2>/dev/null | python3 -c '
import sys, json
d = json.load(sys.stdin)
acts = [a["target"] for a in d["result"]["acts"]]
listed = sys.argv[1]
assert acts, "no act on the board to compare"
for t in acts:
    assert t in listed, f"{t} on the board, not in war sign --list"
rows = [l for l in listed.splitlines() if l.startswith("  ") and l.strip()]
assert len(rows) == len(acts), f"{len(rows)} listed, {len(acts)} on the board"
' "$TUI_LIST" 2>/dev/null; then
    printf 'ok    %-34s board acts == sign --list rows\n' "the queue is war sign --list"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the board and sign --list disagree\n' "the queue is war sign --list"
    FAILED=$((FAILED + 1))
fi
corpus_gone "$TUI_ROOT"

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
