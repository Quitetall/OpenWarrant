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

# t-67ed: the rendered template leads with the ticket loop; the Warrant rules
# follow as the opt-in "When a ticket needs sign-off" and keep every performer
# rule. The same predicate must refuse a template planted performer-first.
am_tickets_first() {
    local t=$1 first tickets signoff performer
    first=$(grep -m1 '^## ' <<<"$t")
    tickets=$(grep -n -m1 '^## Start here: tickets' <<<"$t" | cut -d: -f1)
    signoff=$(grep -n -m1 '^## When a ticket needs sign-off' <<<"$t" | cut -d: -f1)
    performer=$(grep -n -m1 'You are a \*\*performer\*\*' <<<"$t" | cut -d: -f1)
    [[ $first == '## Start here: tickets' ]] || return 1
    [[ -n $tickets && -n $signoff && -n $performer ]] || return 1
    (( tickets < signoff && signoff < performer )) || return 1
    grep -q 'war prime' <<<"$t" && grep -q 'Never ask the human to sign' <<<"$t" \
        && grep -q 'Never verify your own work' <<<"$t" \
        && grep -q 'Never write a disposition you did not receive' <<<"$t" \
        && grep -q 'Never edit a generated file' <<<"$t" \
        && grep -q 'Never change a document to make a tool happy' <<<"$t"
}
AM_TPL=$(./target/debug/war agents-md --stdout)
if am_tickets_first "$AM_TPL"; then
    printf 'ok    %-34s tickets first, sign-off opt-in, rules kept\n' "agents-md leads with tickets"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s template order or a performer rule lost\n' "agents-md leads with tickets"
    FAILED=$((FAILED + 1))
fi
# Plant: the performer section moved above the tickets, as the file opened
# before t-67ed. The predicate must refuse it.
AM_BAD=$(awk 'NR==1{print; print ""; print "## Legacy governed acts: permissions"; print ""; print "You are a **performer**."; next} {print}' <<<"$AM_TPL")
if ! am_tickets_first "$AM_BAD"; then
    printf 'ok    %-34s a performer-first template refused\n' "agents-md order check refuses"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s a performer-first template passed\n' "agents-md order check refuses"
    FAILED=$((FAILED + 1))
fi
