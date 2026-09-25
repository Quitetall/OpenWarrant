# shellcheck shell=bash
# AGENTS.md is written once by `war init` and never silently replaced: an
# adopter may have tuned theirs, and replacing it would be the "change a
# document to make a tool happy" failure the file itself forbids.

plant_cmd "agents-md refuses to clobber without --force" "agents-md.exists" "AGENTS.md" 1 \
    "true" \
    agents-md

# The template renders for this repository byte-for-byte as the committed file:
# one source of rules for this repository's agents and every adopter's.
if diff -q <(./target/debug/war agents-md --stdout) AGENTS.md > /dev/null; then
    printf 'ok    %-34s AGENTS.md is the rendered template\n' "agents-md template matches the file"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s AGENTS.md drifted from the template\n' "agents-md template matches the file"
    FAILED=$((FAILED + 1))
fi
