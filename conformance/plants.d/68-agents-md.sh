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

# M9 (after t-67ed): the rendered template's first sentence says ordinary
# coding needs no Warrant and no ticket; the ticket loop comes next; the rules
# for a Warrant with a sign-off step live in the section scoped to it, and all
# five safety facts are still there, said as what the tool does. The same
# predicate must refuse a template planted performer-first, and one whose
# opening line is a prohibition.
am_ordinary_first() {
    local t=$1 first heading tickets signoff fact
    first=$(grep -v -e '^#' -e '^[[:space:]]*$' <<<"$t" | head -1)
    heading=$(grep -m1 '^## ' <<<"$t")
    tickets=$(grep -n -m1 '^## Tracking work with tickets (optional)' <<<"$t" | cut -d: -f1)
    signoff=$(grep -n -m1 "^## When a Warrant's type requires sign-off" <<<"$t" | cut -d: -f1)
    fact=$(grep -n -m1 'Your own work is checked by someone else' <<<"$t" | cut -d: -f1)
    [[ $first == 'Ordinary coding needs no Warrant and no ticket.'* ]] || return 1
    [[ $heading == '## Tracking work with tickets (optional)' ]] || return 1
    [[ -n $tickets && -n $signoff && -n $fact ]] || return 1
    (( tickets < signoff && signoff < fact )) || return 1
    grep -q 'war view prime' <<<"$t" \
        && grep -q 'No step in this loop needs a signature' <<<"$t" \
        && grep -q 'A disposition comes from the verifier' <<<"$t" \
        && grep -q 'Unknown is reported as UNKNOWN' <<<"$t" \
        && grep -q 'Generated files are rebuilt, not edited' <<<"$t" \
        && grep -q "A checker's input is fixed at its source" <<<"$t" \
        && ! grep -q 'You are a \*\*performer\*\*' <<<"$t"
}
AM_TPL=$(./target/debug/war agents-md --stdout)
if am_ordinary_first "$AM_TPL"; then
    printf 'ok    %-34s ordinary work first, sign-off scoped, five facts kept\n' "agents-md leads with ordinary work"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s template order, opening line or a safety fact lost\n' "agents-md leads with ordinary work"
    FAILED=$((FAILED + 1))
fi
# Plants: the performer section moved above everything, as the file opened
# before t-67ed; and an opening line that forbids work. Each must be refused.
AM_BAD=$(awk 'NR==1{print; print ""; print "## Legacy governed acts: permissions"; print ""; print "You are a **performer**."; next} {print}' <<<"$AM_TPL")
AM_BAD2=$(awk 'NR==3{print "Do not write code without a Warrant."; next} {print}' <<<"$AM_TPL")
if ! am_ordinary_first "$AM_BAD" && ! am_ordinary_first "$AM_BAD2"; then
    printf 'ok    %-34s performer-first and prohibition-first refused\n' "agents-md order check refuses"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s a performer-first or prohibition-first template passed\n' "agents-md order check refuses"
    FAILED=$((FAILED + 1))
fi
unset AM_TPL AM_BAD AM_BAD2
unset -f am_ordinary_first
