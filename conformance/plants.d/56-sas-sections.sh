# shellcheck shell=bash
# SAS sections (OW-WAR-0125, OW-ADR-0028): the document split losslessly and
# outside fences, a generated section index that refuses a hand edit, section
# citations in amendments that resolve at the Warrant's pinned revision, a
# currency per cited section, and `war sas diff` naming sections.
#
# Every claim is paired with the refusal that shows the control can fail, and
# every refusal is matched by the rule AND the detail that fired. Nothing here
# mutates this repository: candidates are written to a temp directory, and the
# plants that edit records do it in a clone of HEAD (full, and shallow for the
# UNKNOWN case), so the clone's history is what the check reads.

SEC_TMP=$(mktemp -d)
SEC_DOC=$(ls docs/sas/*.md | head -1)
SEC_DOC_SHA=$(sha256sum "$SEC_DOC" | cut -d' ' -f1)

sec_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sec_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# The bytes a revision record pins, from this repository's history, by digest.
sec_revision_bytes() {
    local want="$1" out="$2" c
    while read -r c; do
        git show "$c:$SEC_DOC" > "$out" 2>/dev/null || continue
        [[ "$(sha256sum "$out" | cut -d' ' -f1)" == "$want" ]] && return 0
    done < <(git log --full-history --format=%H -- "$SEC_DOC")
    return 1
}

# ---------------------------------------------------------------------------
# OBL-001 — join(split(bytes)) is each recorded revision's pinned sha256.
# `war sas diff` splits the candidate and reports the digest of the join.
for rec in docs/sas/revisions/*.toml; do
    ver=$(sed -n 's/^version = "\(.*\)"/\1/p' "$rec")
    pin=$(sed -n 's/^sha256 = "\(.*\)"/\1/p' "$rec")
    cand="$SEC_TMP/rev-$ver.md"
    if ! sec_revision_bytes "$pin" "$cand"; then
        # Law 15: the bytes are not here, so nothing is claimed about them.
        sec_fail "lossless split of $ver" "UNKNOWN: no commit holds sha256:${pin:0:12}"
        continue
    fi
    out=$("$WAR" sas diff "$cand" 2>&1)
    if grep -q -- "sas.sections.*joined sha256:$pin is the candidate's own" <<<"$out"; then
        sec_ok "lossless split of $ver" "joined sha256:${pin:0:12} = revision record"
    else
        sec_fail "lossless split of $ver" "the join is not sha256:${pin:0:12}"
    fi
done
# The refusal: the same comparison fails for bytes that are not the revision.
cp "$SEC_DOC" "$SEC_TMP/plus-one.md"; printf 'x' >> "$SEC_TMP/plus-one.md"
LATEST_PIN=$(sed -n 's/^sha256 = "\(.*\)"/\1/p' docs/sas/revisions/1.1.0.toml)
out=$("$WAR" sas diff "$SEC_TMP/plus-one.md" 2>&1)
if grep -q -- "joined sha256:" <<<"$out" && ! grep -q -- "joined sha256:$LATEST_PIN" <<<"$out"; then
    sec_ok "one byte more is not revision 1.1.0" "the join names its own digest, not the pin"
else
    sec_fail "one byte more is not revision 1.1.0" "the comparison could not fail"
fi

# A `## 999. Fake` line inside a fence (§105's example list) is not a section;
# the same line outside a fence is. The second is what shows the first is not
# passing because the splitter never sees headings.
python3 - "$SEC_DOC" "$SEC_TMP" <<'PY'
import sys, pathlib
doc, tmp = pathlib.Path(sys.argv[1]).read_text(), pathlib.Path(sys.argv[2])
at = doc.index("## 105. Reference URI forms")
fence = doc.index("```text\n", at) + len("```text\n")
(tmp / "fenced.md").write_text(doc[:fence] + "## 999. Fake\n" + doc[fence:])
before106 = doc.index("## 106. Architecture requirements index")
(tmp / "unfenced.md").write_text(doc[:before106] + "## 999. Fake\n\n" + doc[before106:])
PY
out=$("$WAR" sas diff "$SEC_TMP/fenced.md" 2>&1)
if grep -q -- "sas.diff.section-changed.*section 105 changed" <<<"$out" \
    && ! grep -q -- "section 999" <<<"$out"; then
    sec_ok "a fenced ## 999. is not a section" "§105 changed; no section 999"
else
    sec_fail "a fenced ## 999. is not a section" "section 999 appeared, or §105 was not named"
fi
out=$("$WAR" sas diff "$SEC_TMP/unfenced.md" 2>&1)
if grep -q -- "sas.diff.section-added.*section 999 added" <<<"$out"; then
    sec_ok "an unfenced ## 999. is a section" "rejected as new by sas.diff.section-added"
else
    sec_fail "an unfenced ## 999. is a section" "the splitter did not see it"
fi

# ---------------------------------------------------------------------------
# OBL-005 — `war sas diff` names exactly the sections that changed.
out=$("$WAR" sas diff "$SEC_DOC" 2>&1)
st=$?
if [[ $st -eq 0 ]] && grep -q -- "no section added, removed or changed" <<<"$out" \
    && ! grep -q -- "sas.diff.section-" <<<"$out"; then
    sec_ok "the document against itself" "no section named"
else
    sec_fail "the document against itself" "exit $st, or a section was named"
fi
python3 - "$SEC_DOC" "$SEC_TMP" <<'PY'
import sys, pathlib, re
doc, tmp = pathlib.Path(sys.argv[1]).read_text(), pathlib.Path(sys.argv[2])
start = doc.index("\n### 62.3 ") + 1
end = doc.index("\n### 62.4 ", start)
body = doc[start:end]
first = body.index("\n") + 1          # past the heading line
m = re.search(r"\b[a-z]{4,}\b", body[first:])
i = start + first + m.start()
(tmp / "word-62-3.md").write_text(doc[:i] + "PLANTED" + doc[i + len(m.group(0)):])
rows = doc.split("\n")
row = next(k for k, l in enumerate(rows) if l.startswith("| WAR-SAS-RQ-001 |"))
(tmp / "row-removed.md").write_text("\n".join(rows[:row] + rows[row + 1:]))
PY
out=$("$WAR" sas diff "$SEC_TMP/word-62-3.md" 2>&1)
named=$(grep -c -- "sas.diff.section-" <<<"$out")
if [[ "$named" -eq 1 ]] && grep -q -- "sas.diff.section-changed.*section 62 changed (in 62.3)" <<<"$out"; then
    sec_ok "one word in §62.3" "section 62 named (in 62.3), and no other"
else
    sec_fail "one word in §62.3" "$named section line(s); wanted exactly §62"
fi
out=$("$WAR" sas diff "$SEC_TMP/row-removed.md" 2>&1)
st=$?
if [[ $st -eq 2 ]] && grep -q -- "ERROR sas.diff.removed .*WAR-SAS-RQ-001" <<<"$out"; then
    sec_ok "a §106 row removed" "rejected by sas.diff.removed (WAR-SAS-RQ-001)"
else
    sec_fail "a §106 row removed" "exit $st; sas.diff.removed did not fire for WAR-SAS-RQ-001"
fi

# ---------------------------------------------------------------------------
# OBL-003 / OBL-004 on this corpus: every section citation in an amendment
# resolves at its Warrant's pinned revision and reports its currency.
SEC_REFS=$(grep -hE '^governing_adr_or_policy: "sas://[A-Z]+-SAS-[0-9]' docs/warrants/*/amendments/*.yaml | wc -l)
out=$("$WAR" check 2>&1)
refs_pass=$(grep -c -- "^PASS sas.section-ref " <<<"$out")
refs_bad=$(grep -cE -- "^(ERROR|UNKNOWN|WARN) +sas.section-ref " <<<"$out")
cur_pass=$(grep -c -- "^PASS sas.section-current " <<<"$out")
cur_stale=$(grep -c -- "^WARN sas.section-current " <<<"$out")
cur_bad=$(grep -cE -- "^(ERROR|UNKNOWN) +sas.section-current " <<<"$out")
if [[ "$SEC_REFS" -gt 0 && "$refs_pass" -eq "$SEC_REFS" && "$refs_bad" -eq 0 ]]; then
    sec_ok "existing section citations resolve" "$refs_pass of $SEC_REFS pass sas.section-ref"
else
    sec_fail "existing section citations resolve" "$refs_pass pass, $refs_bad not, of $SEC_REFS"
fi
# OBL-004 says each citation REPORTS its currency, not that every one is
# current: a SAS revision legitimately moves a cited section, and the stale
# citation is then a warning that names it (SAS 1.2.0 moved §37.5 under
# OW-WAR-0114 and 0142). So each citation gets exactly one report, current or
# stale, and none is an error or unknown.
if [[ $((cur_pass + cur_stale)) -eq "$SEC_REFS" && "$cur_bad" -eq 0 ]]; then
    sec_ok "existing section citations report currency" "$cur_pass current, $cur_stale stale (warned), of $SEC_REFS"
else
    sec_fail "existing section citations report currency" "$cur_pass current, $cur_stale stale, $cur_bad error/unknown, of $SEC_REFS"
fi

# A Warrant pinned to 0.1.0-draft.1 by its authorization (no re-pinning
# amendment) whose amendment cites a section: the subject of the edits below.
SEC_ALIAS=""
for am in docs/warrants/*/amendments/AM-001.yaml; do
    d=${am%/amendments/*}
    grep -q '^governing_adr_or_policy: "sas://WAR-SAS-[0-9]' "$am" || continue
    grep -q '^sas_revision = "0.1.0-draft.1"' "$d/authorization.toml" 2>/dev/null || continue
    grep -qh '^sas_revision:' "$d"/amendments/*.yaml && continue
    SEC_ALIAS=${d##*/}
    break
done
if [[ -z "$SEC_ALIAS" ]]; then
    printf 'PLANT SETUP FAILED: no Warrant pinned to 0.1.0-draft.1 cites a section\n' >&2
    exit 9
fi
SEC_AM="docs/warrants/$SEC_ALIAS/amendments/AM-001.yaml"
SEC_CITED=$(sed -n 's/^governing_adr_or_policy: "\(.*\)"/\1/p' "$SEC_AM")

# The clone the record-editing plants run in: HEAD, with its full history.
SEC_CLONE="$SEC_TMP/clone"
git clone -q --no-hardlinks "$REPO_ROOT" "$SEC_CLONE" 2>/dev/null \
    || { printf 'PLANT SETUP FAILED: git clone of HEAD\n' >&2; exit 9; }
PLANT_ROOT="$SEC_CLONE"

# OBL-003 — a citation of nothing is refused, section and subsection alike.
plant "sas://WAR-SAS-999 cites nothing" "ERROR sas.section-ref" "has no section 999" 2 \
    "sed -i 's|$SEC_CITED|sas://WAR-SAS-999|' $SEC_AM; assert_present 'sas://WAR-SAS-999' $SEC_AM" \
    "$SEC_ALIAS"
plant "sas://WAR-SAS-43.9 cites nothing" "ERROR sas.section-ref" "has no subsection 43.9" 2 \
    "sed -i 's|$SEC_CITED|sas://WAR-SAS-43.9|' $SEC_AM; assert_present 'sas://WAR-SAS-43.9' $SEC_AM" \
    "$SEC_ALIAS"
plant "a malformed section citation" "ERROR sas.section-ref" "is not a section reference" 2 \
    "sed -i 's|$SEC_CITED|sas://WAR-SAS-43.5.1|' $SEC_AM; assert_present 'sas://WAR-SAS-43.5.1' $SEC_AM" \
    "$SEC_ALIAS"

# OBL-004 — §98 changed after 0.1.0-draft.1: a citation of it warns, naming
# the pinned revision and the latest ACCEPTED one (derived: it moves as the
# owner accepts revisions). Exit 0: a warning is not a refusal.
SEC_LATEST=$(grep -l '^state = "accepted"' docs/sas/revisions/*.toml | xargs -n1 basename | sed 's/\.toml$//' | sort -V | tail -1)
plant "a cited section that changed" "WARN sas.section-current" \
    "sas://WAR-SAS-98 changed between SAS 0.1.0-draft.1.*SAS $SEC_LATEST" 0 \
    "sed -i 's|$SEC_CITED|sas://WAR-SAS-98|' $SEC_AM; assert_present 'sas://WAR-SAS-98' $SEC_AM" \
    "$SEC_ALIAS"

# OBL-002 — the index is what `war compile` wrote, and a hand edit is drift.
# The revision the document IS: the record whose sha256 is the document's,
# accepted or proposed — not a version written into this plant, which went
# stale the day 1.1.1 was proposed.
SEC_REV=$(grep -l "^sha256 = \"$SEC_DOC_SHA\"" docs/sas/revisions/*.toml 2>/dev/null | head -1 | xargs -r basename | sed 's/\.toml$//')
if [[ -n "$SEC_REV" ]] \
    && grep -q -- "\"revision\":\"$SEC_REV\"" docs/sas/generated/SECTIONS.json \
    && grep -q -- "\"sha256\":\"$SEC_DOC_SHA\"" docs/sas/generated/SECTIONS.json \
    && grep -q -- "sha256:$SEC_DOC_SHA (revision $SEC_REV)" docs/sas/generated/SECTIONS.md; then
    sec_ok "the section index names its source" "sha256:${SEC_DOC_SHA:0:12}, revision $SEC_REV"
else
    sec_fail "the section index names its source" "SECTIONS.* do not name the document and ${SEC_REV:-no recorded revision}"
fi
SEC_ONE=$(python3 -c 'import json; print(json.load(open("docs/sas/generated/SECTIONS.json"))["sections"][40]["sha256"])')
plant "a hand-edited SECTIONS.json is drift" "ERROR sas-sections.drift" "SECTIONS.json" 2 \
    "sed -i 's/$SEC_ONE/${SEC_ONE//[0-9a-f]/0}/' docs/sas/generated/SECTIONS.json; assert_gone '$SEC_ONE' docs/sas/generated/SECTIONS.json" \
    --generated
plant "a hand-edited SECTIONS.md is drift" "ERROR sas-sections.drift" "SECTIONS.md" 2 \
    "printf '| \`999\` | planted | 0–0 | \`0\` |\n' >> docs/sas/generated/SECTIONS.md; assert_present 'planted' docs/sas/generated/SECTIONS.md" \
    --generated
unset PLANT_ROOT

# OBL-004 — in a shallow clone the 0.1.0-draft.1 bytes are not there, and the
# same unchanged citation is UNKNOWN: not the pass the full clone gives it,
# and not a warning.
SEC_SHALLOW="$SEC_TMP/shallow"
git clone -q --depth 1 "file://$REPO_ROOT" "$SEC_SHALLOW" 2>/dev/null \
    || { printf 'PLANT SETUP FAILED: shallow clone of HEAD\n' >&2; exit 9; }
out=$("$WAR" --root "$SEC_SHALLOW" check "$SEC_ALIAS" 2>&1)
st=$?
if [[ $st -eq 2 ]] \
    && grep -q -- "^UNKNOWN sas.section-current .*$SEC_ALIAS.*shallow" <<<"$out" \
    && ! grep -qE -- "^(PASS|WARN) +sas.section-current " <<<"$out"; then
    sec_ok "a shallow clone cannot say" "UNKNOWN sas.section-current ($SEC_CITED, shallow)"
else
    sec_fail "a shallow clone cannot say" "exit $st; wanted UNKNOWN, not pass or warning"
fi

# OBL-006 — the decision is a proposal: OW-ADR-0028's status is `proposed`,
# it governs this Warrant, and its §105 text is marked as a proposal for a
# separate SAS revision. The refusal: a copy whose status is `accepted`.
sec_adr_problems() { # <adr file> -> each way it is not a governing proposal
    local fm
    fm=$(awk 'NR==1 && /^---$/ {f=1; next} f && /^---$/ {exit} f' "$1")
    grep -qx 'status: proposed' <<<"$fm" || echo "status is not proposed"
    grep -qF -- '"war://01a0d04c-5f26-7ba0-a64b-2618b983ba43"' <<<"$fm" || echo "does not govern OW-WAR-0125"
    local flat
    flat=$(tr '\n' ' ' < "$1")
    grep -qE 'proposed wording for a separate SAS +revision' <<<"$flat" || echo "no proposal marking"
}
SEC_ADR=docs/adr/atoms/OW-ADR-0028-sas-sections-are-atoms.md
SEC_ADR_BAD=$(sec_adr_problems "$SEC_ADR" 2>&1)
if [[ -f "$SEC_ADR" && -z "$SEC_ADR_BAD" ]]; then
    sec_ok "OBL-006 OW-ADR-0028 is a proposal" "status proposed; governs OW-WAR-0125; §105 text marked"
else
    sec_fail "OBL-006 OW-ADR-0028 is a proposal" "${SEC_ADR_BAD:-missing}"
fi
sed 's/^status: proposed$/status: accepted/' "$SEC_ADR" > "$SEC_TMP/adr.md"
SEC_ADR_BAD=$(sec_adr_problems "$SEC_TMP/adr.md" 2>&1)
if [[ "$SEC_ADR_BAD" == "status is not proposed" ]]; then
    sec_ok "OBL-006 an accepted ADR is seen" "a copy with status accepted is reported"
else
    sec_fail "OBL-006 an accepted ADR is seen" "reported: ${SEC_ADR_BAD:-nothing}"
fi

# OBL-006 — OW-WAR-0125 changes no SAS byte. The claim is about what this
# Warrant delivered, not about the document today: later revisions (1.1.1,
# 1.2.0) changed the SAS under other Warrants. So: at the commit that first
# delivered the section index, the SAS is byte for byte revision 1.1.0; and
# no deliverable of OW-WAR-0125 names the SAS document. Each is paired with
# the refusal that shows the comparison can fail.
SEC_DELIVERED=$(git log --diff-filter=A --format=%H -- docs/sas/generated/SECTIONS.json | tail -1)
SEC_PIN_110=$(sed -n 's/^sha256 = "\(.*\)"/\1/p' docs/sas/revisions/1.1.0.toml)
SEC_PIN_111=$(sed -n 's/^sha256 = "\(.*\)"/\1/p' docs/sas/revisions/1.1.1.toml)
SEC_AT_DELIVERY=""
[[ -n "$SEC_DELIVERED" ]] \
    && SEC_AT_DELIVERY=$(git show "$SEC_DELIVERED:$SEC_DOC" 2>/dev/null | sha256sum | cut -d' ' -f1)
if [[ -z "$SEC_DELIVERED" || -z "$SEC_PIN_110" ]]; then
    # Law 15: without the history or the record, nothing is claimed.
    sec_fail "OBL-006 the SAS at delivery is 1.1.0" "UNKNOWN: no commit adds SECTIONS.json, or no 1.1.0 record"
elif [[ "$SEC_AT_DELIVERY" == "$SEC_PIN_110" && "$SEC_AT_DELIVERY" != "$SEC_PIN_111" ]]; then
    sec_ok "OBL-006 the SAS at delivery is 1.1.0" "${SEC_DELIVERED:0:8}: sha256:${SEC_AT_DELIVERY:0:12} = revision 1.1.0, not 1.1.1"
else
    sec_fail "OBL-006 the SAS at delivery is 1.1.0" "${SEC_DELIVERED:0:8}: sha256:${SEC_AT_DELIVERY:0:12}, 1.1.0 pins ${SEC_PIN_110:0:12}"
fi

sec_sas_deliverables() { # <deliverables.toml> -> each target_ref naming the SAS document
    sed -n 's/^target_ref = "\(.*\)"/\1/p' "$1" | grep -Fx -- "$SEC_DOC"
}
SEC_OWN=docs/warrants/OW-WAR-0125/deliverables.toml
SEC_HIT=$(sec_sas_deliverables "$SEC_OWN")
SEC_N=$(grep -c '^target_ref = ' "$SEC_OWN" 2>/dev/null)
if [[ -f "$SEC_OWN" && "$SEC_N" -gt 0 && -z "$SEC_HIT" ]]; then
    sec_ok "OBL-006 no deliverable is the SAS" "$SEC_N deliverable(s); none is $SEC_DOC"
else
    sec_fail "OBL-006 no deliverable is the SAS" "$SEC_N deliverable(s); naming the SAS: ${SEC_HIT:-none}"
fi
cp "$SEC_OWN" "$SEC_TMP/deliverables.toml"
printf '\n[[deliverable]]\nid = "D-999"\ntarget_ref = "%s"\n' "$SEC_DOC" >> "$SEC_TMP/deliverables.toml"
SEC_HIT=$(sec_sas_deliverables "$SEC_TMP/deliverables.toml")
if [[ "$SEC_HIT" == "$SEC_DOC" ]]; then
    sec_ok "OBL-006 a SAS deliverable is seen" "a planted D-999 naming the SAS is reported"
else
    sec_fail "OBL-006 a SAS deliverable is seen" "a planted D-999 naming the SAS was not reported"
fi

rm -rf "$SEC_TMP"
