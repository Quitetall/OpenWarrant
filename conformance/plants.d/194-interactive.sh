# shellcheck shell=bash
# W1.7: the human surface. `war` opens the app at a terminal, `--interactive`
# works on all six there, and nothing ever prompts a caller without one. Each
# claim beside its refusal:
#   with no terminal, all six refuse by name, ask nothing and write nothing;
#   so does --json, even at a terminal;
#   at a terminal, `create`, `start`, `done`, `status` and `check` answer from
#   typed lines and do what the plain command does;
#   an empty title creates nothing;
#   bare `war` is the app at a terminal and refused without one.

echo "== the human surface: --interactive (W1.7) =="
IA_T=$(mktemp -d)
ia_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ia_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
IA_WAR="$REPO_ROOT/${WAR#./}"
IA_R="$IA_T/r"; mkdir -p "$IA_R"
ia() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 \
    "$IA_WAR" --root "$IA_R" "$@"; }
# A real terminal: `script` runs the command on a pty and passes these lines in.
ia_tty() { local input="$1"; shift
    printf '%b' "$input" | env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 \
        timeout 30 script -qec "$IA_WAR --root $IA_R $*" /dev/null 2>&1; }
ia init --namespace IA >/dev/null 2>&1 </dev/null

if ! script -qec true /dev/null >/dev/null 2>&1; then
    printf 'UNKNOWN %-32s %s\n' "the human surface" "no pty here: \`script\` cannot run, nothing established"
else
    IA_N0=$(ls "$IA_R/docs/tickets" 2>/dev/null | wc -l)
    IA_BAD=""
    for c in "create --interactive" "start --interactive" "done --interactive" \
        "status --interactive" "check --interactive"; do
        # shellcheck disable=SC2086
        out=$(ia $c 2>&1 </dev/null); rc=$?
        [[ $rc -ne 0 ]] && grep -q 'interactive.no-terminal' <<<"$out" || IA_BAD="$IA_BAD [$c: $rc]"
    done
    out=$(ia init --interactive 2>&1 </dev/null); rc=$?
    [[ $rc -ne 0 ]] && grep -q 'needs a terminal' <<<"$out" || IA_BAD="$IA_BAD [init: $rc]"
    IA_N1=$(ls "$IA_R/docs/tickets" 2>/dev/null | wc -l)
    if [[ -z "$IA_BAD" && $IA_N0 -eq $IA_N1 ]]; then
        ia_ok "no terminal, no prompt" "all six refused by name; nothing written"
    else
        ia_fail "no terminal, no prompt" "$IA_BAD; tickets $IA_N0->$IA_N1"
    fi

    IA_J=$(ia_tty '' --json create --interactive); IA_N2=$(ls "$IA_R/docs/tickets" 2>/dev/null | wc -l)
    if grep -q 'interactive.no-terminal' <<<"$IA_J" && [[ $IA_N2 -eq $IA_N1 ]]; then
        ia_ok "--json never prompts" "refused at a terminal too"
    else
        ia_fail "--json never prompts" "$(head -c 300 <<<"$IA_J")"
    fi

    IA_E=$(ia_tty '\n' create --interactive); IA_N3=$(ls "$IA_R/docs/tickets" 2>/dev/null | wc -l)
    if grep -q 'nothing was created' <<<"$IA_E" && [[ $IA_N3 -eq $IA_N1 ]]; then
        ia_ok "an empty title creates nothing" "said so, no document"
    else
        ia_fail "an empty title creates nothing" "$(head -c 300 <<<"$IA_E")"
    fi

    # title, one step, an end to the steps, a test, no signature.
    IA_O=$(ia_tty 'Typed work\none\n\ntrue\nn\n' create --interactive)
    IA_ID=$(grep -oE 't-[0-9a-f]{4,}' <<<"$IA_O" | head -1)
    IA_L=$(ia --json view warrants </dev/null | python3 -c '
import json, sys
for t in json.load(sys.stdin)["result"]["tickets"]:
    print(t["id"], t["title"])' 2>/dev/null)
    if [[ -f "$IA_R/docs/tickets/$IA_ID.md" ]] && grep -q 'one' "$IA_R/docs/tickets/$IA_ID.md" \
        && grep -q '^run: true' "$IA_R/docs/tickets/$IA_ID.md" && grep -q "$IA_ID Typed work" <<<"$IA_L"; then
        ia_ok "create asks, then does what the flags do" "$IA_ID, its step and its test written"
    else
        ia_fail "create asks, then does what the flags do" "id=$IA_ID: $(tail -c 300 <<<"$IA_O")"
    fi

    IA_S=$(ia_tty '1\n' start --interactive)
    IA_H=$(ia_tty 'finished by hand\n' done --interactive)
    IA_C=$(ia_tty '\n' check --interactive)
    IA_V=$(ia_tty '\n' status --interactive)
    if grep -q "$IA_ID" <<<"$IA_S" && grep -q 'finished by hand' "$IA_R/docs/tickets/$IA_ID/journal.jsonl" \
        && grep -q 'WELL-FORMED\|READY' <<<"$IA_C" && [[ -n "$IA_V" ]]; then
        ia_ok "start, done, check, status ask too" "start took $IA_ID, done kept the note, check and status answered"
    else
        ia_fail "start, done, check, status ask too" "start: $(tail -c 200 <<<"$IA_S") done: $(tail -c 200 <<<"$IA_H") check: $(tail -c 200 <<<"$IA_C")"
    fi

    IA_NT=$(ia 2>&1 </dev/null); IA_NR=$?
    # A new pty has no size, and the app draws nothing into none.
    IA_APP=$( (sleep 2; printf 'q') | env -u SSH_AUTH_SOCK OPENWARRANT_NO_PROJECTS=1 timeout 10 \
        script -qec "stty cols 100 rows 30; $IA_WAR --root $IA_R" /dev/null 2>&1 | wc -c)
    if [[ $IA_NR -eq 2 && $IA_APP -gt 1000 ]]; then
        ia_ok "war is the app at a terminal" "$IA_APP bytes of screen; refused without one, exit 2"
    else
        ia_fail "war is the app at a terminal" "no terminal exit $IA_NR: $(head -c 200 <<<"$IA_NT"); screen $IA_APP bytes"
    fi
fi
command rm -rf "$IA_T"
