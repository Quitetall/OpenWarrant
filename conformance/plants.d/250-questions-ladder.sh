# shellcheck shell=bash
# Questions follow the ladder and find their answerer (OW-ADR-0034).
#
# One scratch repository with no docs/authority/roles.toml: an orchestrator
# ("orch") holds an epic, a worker ("worker") holds a Warrant-level child in a
# worktree of its own, a bystander ("bystander") holds an unrelated ticket, and
# an Order waits on a person. Every claim is paired with its refusal, and each
# refusal is checked for WHICH rule fired and that nothing was written.
#
# Accepted: the worker's question is asked of the orchestrator, who sees it in
# `war status`, `war status --next` (with the exact answer command) and `war
# view prime`, and answers it from the main checkout into the worker's record;
# the worker's next `war` call shows the answer once; a person at a terminal
# answers an Order question and a --needs-human one.
# Refused: the bystander hears nothing; the second call is silent; the asker
# cannot answer itself (question.own); an agent cannot answer an Order or a
# --needs-human question (question.agent); the same person command off a
# terminal is an agent's and is refused.

echo "== questions follow the ladder (OW-ADR-0034) =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID
QL_WAR="$REPO_ROOT/${WAR#./}"
ql_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ql_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
QL_R=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$QL_R")
QL_T="$QL_R/.t"; mkdir -p "$QL_T"
QL_ROOT="$QL_R/repo"; mkdir -p "$QL_ROOT"
ql_g() { git -C "$QL_ROOT" -c commit.gpgsign=false "$@"; }
# ql_as <actor> <checkout> <war args...>: stdout and stderr to $QL_T/out, $QL_T/err.
ql_as() {
    local actor="$1" at="$2"; shift 2
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR="$actor" \
        "$QL_WAR" --root "$at" "$@" >"$QL_T/out" 2>"$QL_T/err" </dev/null
}
# ql_tty <checkout> <war args...>: a person at a terminal, with no actor set.
ql_tty() {
    local at="$1"; shift
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
        timeout 30 script -qec "$QL_WAR --root $at $*" /dev/null >"$QL_T/out" 2>&1 </dev/null
}
ql_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$1"; }
ql_create() { ql_as orch "$QL_ROOT" --json create "$@"; ql_id "$QL_T/out"; }
ql_sum() { sha256sum "$1" 2>/dev/null | cut -d' ' -f1; }

ql_g init -q -b main
ql_g config user.name "Plant Person"
ql_g config user.email plant@invalid
"$QL_WAR" --root "$QL_ROOT" init --namespace QL >/dev/null 2>&1 </dev/null \
    || { printf 'PLANT SETUP FAILED: war init in the questions plant\n' >&2; exit 9; }
[[ ! -e "$QL_ROOT/docs/authority/roles.toml" ]] \
    || { printf 'PLANT SETUP FAILED: war init wrote a register\n' >&2; exit 9; }
QL_EPIC=$(ql_create "The programme")
QL_W=$(ql_create "Pick the store" --part-of "$QL_EPIC" --test "true")
QL_X=$(ql_create "Something else")
QL_O=$(ql_create "Ship the release" --sign)
QL_H=$(ql_create "Name the product")
ql_g add -A; ql_g commit -qm 'tickets'
ql_as orch "$QL_ROOT" admin claim "$QL_EPIC"
ql_as bystander "$QL_ROOT" admin claim "$QL_X"
ql_as worker "$QL_ROOT" --json start --worktree --print "$QL_W"
QL_WT=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["worktree"])' "$QL_T/out" 2>/dev/null)
[[ -d "${QL_WT:-}" ]] || { printf 'PLANT SETUP FAILED: no worktree: %s\n' "$(head -c 300 "$QL_T/err")" >&2; exit 9; }

# ---- 1. the worker asks; the question is asked of the orchestrator ----------
ql_as worker "$QL_WT" note "$QL_W" --ask "sqlite or files?" --blocking
QL_Q="$QL_WT/docs/questions/$QL_W/Q-001.toml"
if grep -q 'asked of orch (holds' "$QL_T/out" && grep -q '^ask_of = "orch"' "$QL_Q" 2>/dev/null; then
    ql_ok "asked of the parent's holder" "Q-001 on $QL_W asked of orch"
else
    ql_fail "asked of the parent's holder" "$(head -c 300 "$QL_T/out") $(head -c 300 "$QL_T/err")"
fi

# ---- 2. it reaches the orchestrator, and no unrelated claim -----------------
ql_as orch "$QL_ROOT" status; QL_S=$(cat "$QL_T/out")
ql_as orch "$QL_ROOT" status --next; QL_N=$(cat "$QL_T/out")
ql_as orch "$QL_ROOT" view prime; QL_P=$(cat "$QL_T/out")
QL_CMD="war plan note $QL_W --answer Q-001"
if [[ "$(head -1 <<<"$QL_S")" == "question for you: $QL_W Q-001"* ]] && grep -qF "$QL_CMD" <<<"$QL_N" \
    && grep -qF "question for you: $QL_W Q-001" <<<"$QL_P"; then
    ql_ok "the orchestrator is told" "status leads with it; next and prime carry \`$QL_CMD\`"
else
    ql_fail "the orchestrator is told" "status: $(head -1 <<<"$QL_S") | next has cmd: $(grep -cF "$QL_CMD" <<<"$QL_N")"
fi
ql_as bystander "$QL_ROOT" status; QL_BS="$(cat "$QL_T/out" "$QL_T/err")"
ql_as bystander "$QL_ROOT" status --next; QL_BN="$(cat "$QL_T/out" "$QL_T/err")"
if ! grep -q "Q-001" <<<"$QL_BS$QL_BN"; then
    ql_ok "an unrelated claim hears nothing" "bystander's status and next name no question"
else
    ql_fail "an unrelated claim hears nothing" "$(grep -m1 Q-001 <<<"$QL_BS$QL_BN")"
fi

# ---- 3. the asker cannot answer itself --------------------------------------
QL_B0=$(ql_sum "$QL_Q")
ql_as worker "$QL_WT" note "$QL_W" --answer Q-001 "files"; QL_RC=$?
if [[ $QL_RC -eq 2 ]] && grep -q 'question.own' "$QL_T/err" && [[ "$(ql_sum "$QL_Q")" == "$QL_B0" ]]; then
    ql_ok "the asker cannot answer" "question.own; the record is byte-identical"
else
    ql_fail "the asker cannot answer" "exit $QL_RC: $(head -c 300 "$QL_T/err")"
fi

# ---- 4. the orchestrator (an agent) answers from the main checkout ----------
ql_as orch "$QL_ROOT" note "$QL_W" --answer Q-001 "files, one per item"; QL_RC=$?
if [[ $QL_RC -eq 0 ]] && grep -q '^answered_by = "agent://orch"' "$QL_Q" && grep -q '^kind = "agent"' "$QL_Q"; then
    ql_ok "an agent answers a Warrant's question" "recorded in the worker's worktree, by agent://orch"
else
    ql_fail "an agent answers a Warrant's question" "exit $QL_RC: $(head -c 300 "$QL_T/err")"
fi
ql_as orch "$QL_ROOT" status
if ! grep -q "Q-001" "$QL_T/out"; then
    ql_ok "an answered question leaves status" "orch's status names it no more"
else
    ql_fail "an answered question leaves status" "$(grep -m1 Q-001 "$QL_T/out")"
fi

# ---- 5. the worker hears the answer once ------------------------------------
ql_as worker "$QL_WT" status
QL_1=$(grep -c "^answer: $QL_W Q-001 .*files, one per item (orch)" "$QL_T/err")
ql_as worker "$QL_WT" status
QL_2=$(grep -c "Q-001" "$QL_T/err" "$QL_T/out" | awk -F: '{s+=$2} END {print s}')
if [[ "$QL_1" == 1 && "$QL_2" == 0 ]]; then
    ql_ok "the asker hears the answer once" "its next call says it; the one after is silent"
else
    ql_fail "the asker hears the answer once" "first $QL_1, second $QL_2"
fi

# ---- 6. a blocking question waits in one line at war start ------------------
ql_as worker "$QL_WT" note "$QL_W" --ask "retry how often?" --blocking
ql_as worker "$QL_WT" start "$QL_W"
QL_WAIT=$(grep -c "^waits on Q-002 (asked of orch)" "$QL_T/out")
ql_as orch "$QL_ROOT" note "$QL_W" --answer Q-002 "three times"
ql_as worker "$QL_WT" start "$QL_W"
if [[ "$QL_WAIT" == 1 ]] && grep -q "^answer: $QL_W Q-002 .*three times (orch)" "$QL_T/out" \
    && ! grep -q "^waits on" "$QL_T/out"; then
    ql_ok "war start says what it waits on" "one line while open; the answer once it comes"
else
    ql_fail "war start says what it waits on" "waits $QL_WAIT; $(tail -2 "$QL_T/out" | tr '\n' '|')"
fi

# ---- 7. an Order needs a person; with no register, a terminal is one --------
ql_as worker "$QL_ROOT" note "$QL_O" --ask "which region?"
QL_OQ="$QL_ROOT/docs/questions/$QL_O/Q-001.toml"
QL_B0=$(ql_sum "$QL_OQ")
ql_as orch "$QL_ROOT" note "$QL_O" --answer Q-001 "eu"; QL_RC=$?
if [[ $QL_RC -eq 2 ]] && grep -q 'question.agent' "$QL_T/err" && grep -q 'is an Order' "$QL_T/err" \
    && [[ "$(ql_sum "$QL_OQ")" == "$QL_B0" ]]; then
    ql_ok "an agent cannot answer an Order" "question.agent naming the Order; nothing written"
else
    ql_fail "an agent cannot answer an Order" "exit $QL_RC: $(head -c 300 "$QL_T/err")"
fi
if ! script -qec true /dev/null >/dev/null 2>&1; then
    printf 'UNKNOWN %-32s %s\n' "a person answers an Order" "no pty here: \`script\` cannot run, nothing established"
else
    # Off a terminal, the same command is an agent's: refused.
    env -u OPENWARRANT_ACTOR "$QL_WAR" --root "$QL_ROOT" note "$QL_O" --answer Q-001 eu-west \
        >"$QL_T/out" 2>"$QL_T/err" </dev/null; QL_RC=$?
    QL_OFF="$QL_RC $(grep -c question.agent "$QL_T/err")"
    ql_tty "$QL_ROOT" note "$QL_O" --answer Q-001 eu-west
    if [[ "$QL_OFF" == "2 1" ]] && grep -q 'answered by Plant Person (person)' "$QL_T/out" \
        && grep -q '^answered_by = "person://Plant Person"' "$QL_OQ"; then
        ql_ok "a person answers an Order" "at a terminal, as the git identity; off one, refused"
    else
        ql_fail "a person answers an Order" "off a terminal: $QL_OFF; $(head -c 300 "$QL_T/out")"
    fi
    # And a harness session at a terminal is still an agent's.
    ql_as worker "$QL_ROOT" note "$QL_H" --ask "what name?" --needs-human
    QL_HQ="$QL_ROOT/docs/questions/$QL_H/Q-001.toml"
    QL_B0=$(ql_sum "$QL_HQ")
    ql_as orch "$QL_ROOT" note "$QL_H" --answer Q-001 "Warrant"; QL_RC=$?
    QL_AG="$QL_RC $(grep -c 'question.agent' "$QL_T/err") $(grep -c 'needs-human' "$QL_T/err")"
    CLAUDE_CODE_SESSION_ID=plant ql_tty "$QL_ROOT" note "$QL_H" --answer Q-001 Warrant
    QL_SESS=$(grep -c 'question.agent' "$QL_T/out")
    if [[ "$QL_AG" == "2 1 1" && "$QL_SESS" == 1 && "$(ql_sum "$QL_HQ")" == "$QL_B0" ]]; then
        ql_ok "--needs-human refuses an agent" "question.agent, at a terminal too in a harness session"
    else
        ql_fail "--needs-human refuses an agent" "agent: $QL_AG; session at a pty: $QL_SESS"
    fi
    ql_tty "$QL_ROOT" note "$QL_H" --answer Q-001 Warrant
    if grep -q '^kind = "person"' "$QL_HQ"; then
        ql_ok "a person answers --needs-human" "recorded as a person's"
    else
        ql_fail "a person answers --needs-human" "$(head -c 300 "$QL_T/out")"
    fi
fi
