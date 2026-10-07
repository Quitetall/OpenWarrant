# shellcheck shell=bash
# OW-WAR-0112, M9 — `war init` asks nothing; the conversation is opt-in.
#
# The guided machine itself is unit-tested (init::guided::tests) with canned
# answers. These plants hold what a shell can see: plain `war init` derives
# its namespace and writes no signing setup, `--guided` without a terminal is
# refused with nothing written, and the scripted scaffold is byte-for-byte
# what it always was.

echo "== init asks nothing; guided is opt-in (OW-WAR-0112, M9) =="
GI_TMP=$(mktemp -d)
WAR_ABS=$(cd "$(dirname "$WAR")" && pwd)/$(basename "$WAR")

# M9: no --namespace, no terminal: a namespace derived from the directory
# name, the plain files, and no signing scaffolding (no docs/authority/, no
# SAS, no first Warrant).
GI_PLAINDIR="$GI_TMP/my-game-engine"
mkdir -p "$GI_PLAINDIR"
GI_OUT=$(cd "$GI_PLAINDIR" && git init -q . && "$WAR_ABS" init </dev/null 2>&1)
GI_STATUS=$?
GI_SAS=$(find "$GI_PLAINDIR/docs/sas" -type f 2>/dev/null | head -1)
GI_WARRANT=$(find "$GI_PLAINDIR/docs/warrants" -mindepth 1 -maxdepth 1 2>/dev/null | head -1)
if [[ $GI_STATUS -eq 0 ]] && grep -q '^namespace MGE, from the directory name' <<<"$GI_OUT" \
    && grep -q '^namespace = "MGE"$' "$GI_PLAINDIR/openwarrant.toml" \
    && [[ -f "$GI_PLAINDIR/AGENTS.md" ]] && [[ ! -e "$GI_PLAINDIR/docs/authority" ]] \
    && [[ -z $GI_SAS && -z $GI_WARRANT ]]; then
    printf 'ok    %-34s namespace MGE derived, no signing setup\n' "init without a terminal asks nothing"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; %s; sas=%s warrant=%s\n' "init without a terminal asks nothing" "$GI_STATUS" "$(tr '\n' '|' <<<"$GI_OUT")" "$GI_SAS" "$GI_WARRANT"
    FAILED=$((FAILED + 1))
fi

# The old spelling `--non-interactive` keeps working: it now means what plain
# `war init` does.
GI_NIDIR="$GI_TMP/NonInteractive"
mkdir -p "$GI_NIDIR"
GI_NI=$(cd "$GI_NIDIR" && git init -q . && "$WAR_ABS" init --non-interactive </dev/null 2>&1)
if [[ $? -eq 0 ]] && grep -q '^namespace = "NI"$' "$GI_NIDIR/openwarrant.toml"; then
    printf 'ok    %-34s accepted, namespace NI derived\n' "--non-interactive still works"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "--non-interactive still works" "$(tr '\n' '|' <<<"$GI_NI")"
    FAILED=$((FAILED + 1))
fi

# The refusals it pairs with: `--guided` without a terminal is refused by
# name and writes nothing, and an explicit bad `--namespace` is still refused.
GI_GDIR="$GI_TMP/guided"
mkdir -p "$GI_GDIR"
GI_OUT2=$(cd "$GI_GDIR" && git init -q . && "$WAR_ABS" init --guided </dev/null 2>&1)
GI_S2=$?
GI_OUT2B=$(cd "$GI_GDIR" && "$WAR_ABS" init --namespace lower </dev/null 2>&1)
GI_S2B=$?
if [[ $GI_S2 -ne 0 ]] && grep -q 'needs a terminal' <<<"$GI_OUT2" \
    && [[ $GI_S2B -ne 0 ]] && [[ ! -e "$GI_GDIR/openwarrant.toml" ]] \
    && [[ ! -e "$GI_GDIR/docs/authority/roles.toml" ]]; then
    printf 'ok    %-34s refused (exit %s, %s), nothing written\n' "--guided needs a terminal" "$GI_S2" "$GI_S2B"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s/%s; %s\n' "--guided needs a terminal" "$GI_S2" "$GI_S2B" "$(tr '\n' '|' <<<"$GI_OUT2 $GI_OUT2B")"
    FAILED=$((FAILED + 1))
fi

# The scripted scaffold: examples only, never the real authority files, and
# the three lines it always printed.
GI_OUT3=$(cd "$GI_TMP" && git init -q . && "$WAR_ABS" init --program "Plant Program" --namespace PP </dev/null 2>&1)
if [[ $? -eq 0 ]] \
    && [[ -f "$GI_TMP/docs/authority/roles.toml.example" ]] \
    && [[ -f "$GI_TMP/docs/authority/allowed_signers.example" ]] \
    && [[ ! -e "$GI_TMP/docs/authority/roles.toml" ]] \
    && [[ ! -e "$GI_TMP/docs/authority/allowed_signers" ]] \
    && [[ $(wc -l <<<"$GI_OUT3") -eq 3 ]] \
    && grep -q '^initialized Plant Program' <<<"$GI_OUT3" \
    && grep -q '^scaffolded Plant Program: ' <<<"$GI_OUT3" \
    && grep -q '^next: edit the SAS, `war sas propose 0.1.0`' <<<"$GI_OUT3"; then
    printf 'ok    %-34s examples only, three lines\n' "scripted scaffold unchanged"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "scripted scaffold unchanged" "$(tr '\n' '|' <<<"$GI_OUT3")"
    FAILED=$((FAILED + 1))
fi

# The examples the scaffold ships state the rule the tool now follows.
if grep -q 'a tool writes this file only' "$GI_TMP/docs/authority/roles.toml.example" \
    && grep -q 'war init' "$GI_TMP/docs/authority/allowed_signers.example"; then
    printf 'ok    %-34s both examples state the write-once rule\n' "examples state the rule"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the shipped examples do not state the rule\n' "examples state the rule"
    FAILED=$((FAILED + 1))
fi

# t-67ed, M9: plain `war init`'s first hint, the line straight after
# `initialized`, says ordinary coding needs no ticket and no Warrant, then
# how to track work if wanted; the scripted `--program` scaffold above never
# prints it (its three lines are pinned).
GI_PLAIN=$(mktemp -d)
GI_OUT4=$(cd "$GI_PLAIN" && git init -q . && "$WAR_ABS" init --namespace TK </dev/null 2>&1)
if [[ $? -eq 0 ]] && [[ $(sed -n 1p <<<"$GI_OUT4") == initialized* ]] \
    && [[ $(sed -n 2p <<<"$GI_OUT4") == 'start: ordinary coding needs no ticket and no Warrant.'* ]] \
    && grep -q 'war create ' <<<"$GI_OUT4" && grep -q 'no signature needed' <<<"$GI_OUT4" \
    && ! grep -q '^namespace ' <<<"$GI_OUT4"; then
    printf 'ok    %-34s the first hint is the ticket start\n' "plain init leads with tickets"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "plain init leads with tickets" "$(tr '\n' '|' <<<"$GI_OUT4")"
    FAILED=$((FAILED + 1))
fi
if ! grep -q '^start:' <<<"$GI_OUT3"; then
    printf 'ok    %-34s no start line in the pinned three\n' "scaffold output keeps its shape"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the --program scaffold printed a start line\n' "scaffold output keeps its shape"
    FAILED=$((FAILED + 1))
fi
rm -rf "$GI_PLAIN"
rm -rf "$GI_TMP"
