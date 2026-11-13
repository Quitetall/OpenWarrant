# shellcheck shell=bash
# AGENTS.md is written once by `war init` and never silently replaced: an
# adopter may have tuned theirs, and replacing it would be the "change a
# document to make a tool happy" failure the file itself forbids.

plant_cmd "agents-md refuses to clobber without --force" "agents-md.exists" "AGENTS.md" 1 \
    "true" \
    agents-md

# The template is short (at most 4,000 bytes: an agent reads it every
# session) and links the full rules, which the legacy reference keeps.
# Refusal: the same predicate fails a template padded past the budget.
LEGACY_AGENT_REFERENCE="docs/agents/legacy-warrant-workflow.md"
am_short_and_linked() {
    local t=$1
    (( ${#t} <= 4000 )) && grep -Fq -- "$LEGACY_AGENT_REFERENCE" <<<"$t" \
        && grep -Fq '### Writing the atoms' "$LEGACY_AGENT_REFERENCE" \
        && grep -Fq 'war sign resolve --dry-run <alias>' "$LEGACY_AGENT_REFERENCE"
}
AM_SHORT=$(./target/debug/war agents-md --stdout)
AM_PADDED="$AM_SHORT$(printf '%04000d' 0)"
if am_short_and_linked "$AM_SHORT" && ! am_short_and_linked "$AM_PADDED"; then
    printf 'ok    %-34s %s bytes, links the full rules; a padded one refused\n' "agents-md short and linked" "${#AM_SHORT}"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s bytes, or the link or the full rules lost\n' "agents-md short and linked" "${#AM_SHORT}"
    FAILED=$((FAILED + 1))
fi
unset AM_SHORT AM_PADDED
unset -f am_short_and_linked

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
    tickets=$(grep -n -m1 '^## Tracking work (optional)' <<<"$t" | cut -d: -f1)
    signoff=$(grep -n -m1 "^## Formal profile requirements" <<<"$t" | cut -d: -f1)
    fact=$(grep -n -m1 'Your own work is checked by someone else' <<<"$t" | cut -d: -f1)
    [[ $first == 'Ordinary coding needs no Warrant and no ticket.'* ]] || return 1
    [[ $heading == '## Tracking work (optional)' ]] || return 1
    [[ -n $tickets && -n $signoff && -n $fact ]] || return 1
    (( tickets < signoff && signoff < fact )) || return 1
    grep -q 'war view prime' <<<"$t" \
        && grep -q 'Workers finish their portion without signing' <<<"$t" \
        && grep -q 'leaves the work awaiting signature' <<<"$t" \
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
