# shellcheck shell=bash
# The hotline (OW-WAR-0069): an agent asks, a human answers, and the tool
# refuses the reverse. Every plant runs against OW-WAR-0069, whose STAGE-001
# is the grilling these questions belong to; anything the plants create under
# its questions/ is removed here, since `git checkout` does not undo an
# untracked file.

Q_ALIAS=OW-WAR-0069
Q_DIR=docs/warrants/$Q_ALIAS/questions
Q_BEFORE=$(ls "$Q_DIR" 2>/dev/null | sort)

q_cleanup() {
    local f base
    for f in "$Q_DIR"/*.toml; do
        [[ -e "$f" ]] || continue
        base=$(basename "$f")
        grep -qxF "$base" <<< "$Q_BEFORE" || rm -f "$f"
    done
    restore
}

# An agent answering its own question: refused by actor kind, nothing written.
Q_ASK=$("$WAR" ask "$Q_ALIAS" STAGE-001 "Planted: may an agent answer this?" --blocking 2>&1)
Q_ID=$(grep -o 'Q-[0-9]\{3\}' <<< "$Q_ASK" | head -1)
if [[ -z "$Q_ID" ]]; then
    printf 'FAIL  %-34s could not ask a question to plant against\n' "the hotline plants"
    FAILED=$((FAILED + 1))
else
    out=$("$WAR" answer "$Q_ALIAS" "$Q_ID" "I, an agent, answer myself" --as claude 2>&1)
    status=$?
    if [[ $status -eq 2 ]] && grep -q 'question.agent' <<< "$out" && ! grep -qE '^answered_by' "$Q_DIR/$Q_ID.toml"; then
        printf 'ok    %-34s rejected by question.agent, nothing written\n' "an agent answering"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s exit %s\n%s\n' "an agent answering" "$status" "$(head -3 <<< "$out")"
        FAILED=$((FAILED + 1))
    fi

    # An actor with no role assignment: refused by name.
    out=$("$WAR" answer "$Q_ALIAS" "$Q_ID" "x" --as nobody 2>&1)
    if [[ $? -eq 2 ]] && grep -q 'question.unknown-actor' <<< "$out"; then
        printf 'ok    %-34s rejected by question.unknown-actor\n' "an unregistered answerer"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "an unregistered answerer" "$(head -2 <<< "$out")"
        FAILED=$((FAILED + 1))
    fi

    # The positive: the human in the register answers, the record carries it,
    # and the stage's performer can read it back.
    out=$("$WAR" answer "$Q_ALIAS" "$Q_ID" "Yes, a human answers." --as "Brian Lam" 2>&1)
    if [[ $? -eq 0 ]] && grep -q 'question.answered' <<< "$out" \
        && grep -qE '^answered_by = "person://Brian Lam"' "$Q_DIR/$Q_ID.toml" \
        && Q_ANSWERS=$("$WAR" answers "$Q_ALIAS" STAGE-001 2>&1) && grep -q 'Yes, a human answers.' <<<"$Q_ANSWERS"; then
        printf 'ok    %-34s recorded, and war answers reads it back\n' "a human answering"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "a human answering" "$(head -2 <<< "$out")"
        FAILED=$((FAILED + 1))
    fi

    # A second answer is refused: an answer is history, not a field to edit.
    out=$("$WAR" answer "$Q_ALIAS" "$Q_ID" "Actually, no." --as "Brian Lam" 2>&1)
    if [[ $? -eq 2 ]] && grep -q 'question.answered' <<< "$out" \
        && ! grep -q 'Actually, no.' "$Q_DIR/$Q_ID.toml"; then
        printf 'ok    %-34s rejected; the first answer stands\n' "answering twice"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "answering twice" "$(head -2 <<< "$out")"
        FAILED=$((FAILED + 1))
    fi
fi
q_cleanup

# A question against a stage the Warrant does not declare: refused by name.
plant_cmd "a question against no stage" "question.unknown-stage" "STAGE-999" 1 \
    "true" ask "$Q_ALIAS" STAGE-999 "Planted: who reads this?"
q_cleanup

# A question with no text cannot be answered, so it is not accepted.
plant_cmd "an empty question" "question.empty" "" 1 \
    "true" ask "$Q_ALIAS" STAGE-001 "   "
q_cleanup

# Positive: bring an open question; committed ones may all be answered.
# The listing still puts blocking questions first and gives an answer command.
plant_cmd "open questions list blocking first" "BLOCKING" "war plan answer" 0 \
    "\"$WAR\" ask \"$Q_ALIAS\" STAGE-001 \"Planted: open listing?\" --blocking >/dev/null 2>&1 || exit 9" questions --open
q_cleanup
# And the watcher raises them beside the pending signatures.
plant_cmd "watch raises a question" "question" "$Q_ALIAS" 0 \
    "\"$WAR\" ask \"$Q_ALIAS\" STAGE-001 \"Planted: watcher question?\" --blocking >/dev/null 2>&1 || exit 9" watch --once
q_cleanup

# A record that does not parse hides nothing: it is named, and the rest of the
# queue still lists (a corpus-wide queue that one typo empties is worse than
# one that reports the typo).
# Allocate both fixture ids through the public command; never overwrite a
# pre-existing record just because its number looks unlikely.
Q_ASK=$("$WAR" ask "$Q_ALIAS" STAGE-001 "Planted: keep listing this question." --blocking 2>&1) || { echo "PLANT SETUP FAILED: visible question" >&2; exit 9; }
Q_VISIBLE_ID=$(grep -o 'Q-[0-9]\{3\}' <<< "$Q_ASK" | head -1)
Q_ASK=$("$WAR" ask "$Q_ALIAS" STAGE-001 "Planted: malformed record." 2>&1) || { echo "PLANT SETUP FAILED: malformed-question fixture" >&2; exit 9; }
Q_BAD_ID=$(grep -o 'Q-[0-9]\{3\}' <<< "$Q_ASK" | head -1)
if [[ -z "$Q_VISIBLE_ID" || -z "$Q_BAD_ID" || "$Q_VISIBLE_ID" == "$Q_BAD_ID" ]] \
    || [[ ! -f "$Q_DIR/$Q_VISIBLE_ID.toml" || ! -f "$Q_DIR/$Q_BAD_ID.toml" ]] \
    || grep -qxF "$Q_BAD_ID.toml" <<< "$Q_BEFORE" \
    || grep -qxF "$Q_VISIBLE_ID.toml" <<< "$Q_BEFORE"; then
    printf 'PLANT SETUP FAILED: cannot allocate two fresh question records\n' >&2
    exit 9
fi
printf 'not a question [[[\n' > "$Q_DIR/$Q_BAD_ID.toml"
out=$("$WAR" questions --open 2>&1); status=$?
if [[ $status -eq 2 ]] && grep -q 'question.malformed' <<< "$out" && grep -Fq "$Q_VISIBLE_ID" <<< "$out"; then
    printf 'ok    %-34s named, and the other questions still list\n' "a malformed question record"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s\n%s\n' "a malformed question record" "$status" "$(head -3 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
# `ask` is strict about the same file: it allocates the next id from the set.
out=$("$WAR" ask "$Q_ALIAS" STAGE-001 "Planted: strict?" 2>&1)
if [[ $? -ne 0 ]] && grep -q 'not a question record' <<< "$out"; then
    printf 'ok    %-34s refuses to allocate an id over an unreadable set\n' "ask with a malformed record"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "ask with a malformed record" "$(head -2 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
q_cleanup
