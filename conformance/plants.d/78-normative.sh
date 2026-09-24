# shellcheck shell=bash
# The SAS normative projection (slice E1): NORMATIVE.{md,json} are compiled
# from the document and drift-checked like every other projection.

SAS_DOC=$(ls docs/sas/*.md | head -1)

# A SHALL sentence edited in the document without recompiling is drift, in
# both projections, and the rule names the file.
plant "a SHALL edited without recompiling is drift" "sas-normative.drift" "NORMATIVE" 2 \
    "sed -i '0,/The compiler SHALL:/s//The compiler SHALL always:/' \"$SAS_DOC\"; assert_present 'SHALL always:' \"$SAS_DOC\"" \
    --generated

# The projection carries a recorded revision's digest (positive): the sha256
# in NORMATIVE.json is one a revision record under docs/sas/revisions/ names —
# the accepted one, or the proposal in flight the document currently is.
NORM_SHA=$(python3 -c 'import json; print(json.load(open("docs/sas/generated/NORMATIVE.json"))["sha256"])')
REV_SHA=$(grep -h '^sha256' docs/sas/revisions/*.toml | cut -d'"' -f2 | grep -Fx -- "$NORM_SHA" | head -1)
if [[ -n "$NORM_SHA" && "$NORM_SHA" == "$REV_SHA" ]]; then
    printf 'ok    %-34s sha256:%s is a recorded revision\n' "the projection carries the SAS digest" "${NORM_SHA:0:12}"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s projection %s, revision %s\n' "the projection carries the SAS digest" "$NORM_SHA" "$REV_SHA"
    FAILED=$((FAILED + 1))
fi

# The projection is what the committed tree says it is: a hand edit is drift.
plant "a hand-edited NORMATIVE.md is drift" "sas-normative.drift" "NORMATIVE.md" 2 \
    "printf '\n- **SHALL** §999: planted.\n' >> docs/sas/generated/NORMATIVE.md; assert_present '§999' docs/sas/generated/NORMATIVE.md" \
    --generated

# Completeness, which drift cannot see. A section heading the parser cannot
# label drops every sentence under it, identically on every compilation, so the
# drift check passes over it. Knowledge Fabric lost 28 of 162 requirements to
# that for weeks (five lettered sections, reported 2026-09-16).
plant "an unlabelable section is reported" "sas-normative.section-dropped" "8-bis" 2 \
    "printf '\n## 8-bis. Planted later thoughts\n\nThe reader SHALL NOT recurse.\n' >> \"\$SAS_DOC\"; assert_present '8-bis' \"\$SAS_DOC\"" \
    --generated

# The positive that makes the rule worth having: a LETTERED section is a
# section, its sentences reach the projection, and nothing is reported.
NORM_TMP=$(mktemp -d)
printf '## 8A. Planted appended section\n\nThe reader SHALL admit one planted item.\n' >> "$SAS_DOC"
"$WAR" compile > /dev/null 2>&1
if grep -Fq 'planted item' docs/sas/generated/NORMATIVE.md \
    && ! "$WAR" check --generated 2>&1 | grep -q 'section-dropped'; then
    printf 'ok    %-34s §8A projects, nothing reported\n' "a lettered section is a section"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s §8A did not reach the projection\n' "a lettered section is a section"
    FAILED=$((FAILED + 1))
fi
rm -rf "$NORM_TMP"
restore
"$WAR" compile > /dev/null 2>&1
