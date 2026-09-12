# shellcheck shell=bash
# The document work kind (slice C4a): `war document review` on the example
# memo of OW-WAR-0065. Its true state is exit 2 for `document.review-absent`
# until an independent verifier records `established`; every other rule is
# planted against a memo that is otherwise sound.

MEMO=docs/research/tokenizer-approximation.md

# The true state: the memo is at its digest, cites what exists, has no
# placeholder, and awaits a reviewer who is not the author.
plant_cmd "an unreviewed memo is refused only for its review" "document.review-absent" "OBL-002" 2 \
    "true" \
    document review OW-WAR-0065
OUT=$("$WAR" document review OW-WAR-0065 2>&1)
if grep -q 'document.digested' <<< "$OUT" && ! grep -qE 'document.(placeholder|citation-missing|undigested)' <<< "$OUT"; then
    printf 'ok    %-34s digest holds, citations resolve, no placeholder\n' "the memo passes the other three rules"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s\n%s\n' "the memo passes the other three rules" "$(grep -E '^ERROR' <<< "$OUT" | head -3)"
    FAILED=$((FAILED + 1))
fi

plant_cmd "a TODO left in the memo is refused" "document.placeholder" "TODO" 2 \
    "printf '\nTODO: check this figure.\n' >> $MEMO; assert_present 'TODO: check' $MEMO" \
    document review OW-WAR-0065

plant_cmd "a byte moved after the digest is refused" "document.undigested" "has moved" 2 \
    "printf '\n' >> $MEMO; assert_present 'bytes-div-4' $MEMO" \
    document review OW-WAR-0065

plant_cmd "a citation to a missing file is refused" "document.citation-missing" "no/such/source.md" 2 \
    "sed -i 's|(../../crates/openwarrant-core/src/tokens.rs)|(no/such/source.md)|' $MEMO; assert_present 'no/such/source.md' $MEMO" \
    document review OW-WAR-0065

# Corpus-wide, only Warrants that bind to this gate are reviewed; the code
# Warrants that happen to deliver Markdown are not its contract.
plant_cmd "the corpus review is scoped to its binders" "document.reviewed-warrants" "1 Warrant(s)" 2 \
    "true" \
    document review
