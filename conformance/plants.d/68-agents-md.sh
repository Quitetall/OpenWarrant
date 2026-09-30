# shellcheck shell=bash
# AGENTS.md is written once by `war init` and never silently replaced: an
# adopter may have tuned theirs, and replacing it would be the "change a
# document to make a tool happy" failure the file itself forbids.

plant_cmd "agents-md refuses to clobber without --force" "agents-md.exists" "AGENTS.md" 1 \
    "true" \
    agents-md

# The generic legacy template stays byte-for-byte equal to the linked workflow
# reference. Root AGENTS.md retains this repository's own context and routing.
LEGACY_AGENT_REFERENCE="docs/agents/legacy-warrant-workflow.md"
if diff -q <(./target/debug/war agents-md --stdout) "$LEGACY_AGENT_REFERENCE" > /dev/null; then
    printf 'ok    %-34s legacy reference is the rendered template\n' "agents-md template matches reference"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s legacy reference drifted from the template\n' "agents-md template matches reference"
    FAILED=$((FAILED + 1))
fi

if grep -Fq -- "]($LEGACY_AGENT_REFERENCE)" AGENTS.md; then
    printf 'ok    %-34s root instructions link the legacy workflow\n' "agents-md preserves root routing"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s root instructions lost the legacy workflow link\n' "agents-md preserves root routing"
    FAILED=$((FAILED + 1))
fi
