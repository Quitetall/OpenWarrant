# shellcheck shell=bash
# OW-WAR-0112 — `war init` is a conversation, and only at a terminal.
#
# The machine itself is unit-tested (init::guided::tests) with canned
# answers. These plants hold the two invariants a shell can see: without a
# terminal nothing asks and nothing authoritative is written, and the
# scripted form is byte-for-byte what it always was.

echo "== guided init (OW-WAR-0112) =="
GI_TMP=$(mktemp -d)
WAR_ABS=$(cd "$(dirname "$WAR")" && pwd)/$(basename "$WAR")

# No --namespace, no terminal: a refusal naming the flag, no prompt, no files.
GI_OUT=$(cd "$GI_TMP" && git init -q . && "$WAR_ABS" init </dev/null 2>&1)
GI_STATUS=$?
if [[ $GI_STATUS -ne 0 ]] && grep -q -- '--namespace' <<<"$GI_OUT" \
    && [[ ! -e "$GI_TMP/openwarrant.toml" ]] && [[ ! -e "$GI_TMP/docs/authority/roles.toml" ]]; then
    printf 'ok    %-34s refused (exit %s), nothing written\n' "init without a terminal asks nothing" "$GI_STATUS"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; %s\n' "init without a terminal asks nothing" "$GI_STATUS" "$(ls "$GI_TMP")"
    FAILED=$((FAILED + 1))
fi

# --non-interactive says the same thing, so a script can pin the behaviour.
GI_OUT2=$(cd "$GI_TMP" && "$WAR_ABS" init --non-interactive </dev/null 2>&1)
if [[ $? -ne 0 ]] && grep -q -- '--namespace' <<<"$GI_OUT2"; then
    printf 'ok    %-34s refused, names --namespace\n' "--non-interactive never asks"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "--non-interactive never asks" "$GI_OUT2"
    FAILED=$((FAILED + 1))
fi

# The scripted scaffold: examples only, never the real authority files, and
# the three lines it always printed.
GI_OUT3=$(cd "$GI_TMP" && "$WAR_ABS" init --program "Plant Program" --namespace PP </dev/null 2>&1)
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

rm -rf "$GI_TMP"
