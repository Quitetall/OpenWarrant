# shellcheck shell=bash
# The progress platform (slices D3/D4): the committed page inlines the three
# projections, reaches for nothing, and every route renders against the
# committed data (a stub DOM under node; skipped, and said, without node).

PAGE=docs/warrants/generated/CORPUS_STATUS.html
APP=crates/openwarrant-compiler/src/corpus_status/app.js

if grep -q 'id="corpus-timeline"' "$PAGE" && grep -q 'id="corpus-pending"' "$PAGE" && grep -q 'id="corpus-status"' "$PAGE"; then
    printf 'ok    %-34s status, timeline and pending are inlined\n' "the page carries three projections"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s a projection is missing from the page\n' "the page carries three projections"
    FAILED=$((FAILED + 1))
fi

if command -v node >/dev/null 2>&1; then
    if OUT=$(node conformance/fixtures/platform/render-routes.js "$PAGE" "$APP" 2>&1); then
        printf 'ok    %-34s %s\n' "every route renders" "$(tail -1 <<< "$OUT")"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s\n%s\n' "every route renders" "$(grep -E '^FAIL|routes rendered' <<< "$OUT")"
        FAILED=$((FAILED + 1))
    fi
else
    printf 'skip  %-34s node is not installed; the route harness did not run\n' "every route renders"
fi
