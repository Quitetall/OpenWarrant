# shellcheck shell=bash
# The SAS normative projection (slice E1): NORMATIVE.{md,json} are compiled
# from the document and drift-checked like every other projection.

SAS_DOC=$(ls docs/sas/*.md | head -1)

# A SHALL sentence edited in the document without recompiling is drift, in
# both projections, and the rule names the file.
plant "a SHALL edited without recompiling is drift" "sas-normative.drift" "NORMATIVE" 2 \
    "sed -i '0,/The compiler SHALL:/s//The compiler SHALL always:/' \"$SAS_DOC\"; assert_present 'SHALL always:' \"$SAS_DOC\"" \
    --generated

# The projection carries the accepted revision's digest (positive): the
# sha256 in NORMATIVE.json equals the one the accepted revision records.
NORM_SHA=$(python3 -c 'import json; print(json.load(open("docs/sas/generated/NORMATIVE.json"))["sha256"])')
REV_SHA=$(grep '^sha256' docs/sas/revisions/0.1.0-draft.3.toml | cut -d'"' -f2)
if [[ -n "$NORM_SHA" && "$NORM_SHA" == "$REV_SHA" ]]; then
    printf 'ok    %-34s sha256:%s is the accepted revision\n' "the projection carries the SAS digest" "${NORM_SHA:0:12}"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s projection %s, revision %s\n' "the projection carries the SAS digest" "$NORM_SHA" "$REV_SHA"
    FAILED=$((FAILED + 1))
fi

# The projection is what the committed tree says it is: a hand edit is drift.
plant "a hand-edited NORMATIVE.md is drift" "sas-normative.drift" "NORMATIVE.md" 2 \
    "printf '\n- **SHALL** §999: planted.\n' >> docs/sas/generated/NORMATIVE.md; assert_present '§999' docs/sas/generated/NORMATIVE.md" \
    --generated
