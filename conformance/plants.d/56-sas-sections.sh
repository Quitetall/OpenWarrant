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
cur_bad=$(grep -cE -- "^(ERROR|UNKNOWN|WARN) +sas.section-current " <<<"$out")
if [[ "$SEC_REFS" -gt 0 && "$refs_pass" -eq "$SEC_REFS" && "$refs_bad" -eq 0 ]]; then
    sec_ok "existing section citations resolve" "$refs_pass of $SEC_REFS pass sas.section-ref"
else
    sec_fail "existing section citations resolve" "$refs_pass pass, $refs_bad not, of $SEC_REFS"
fi
if [[ "$cur_pass" -eq "$SEC_REFS" && "$cur_bad" -eq 0 ]]; then
    sec_ok "existing section citations current" "$cur_pass of $SEC_REFS pass sas.section-current"
else
    sec_fail "existing section citations current" "$cur_pass pass, $cur_bad not, of $SEC_REFS"
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

# OBL-004 — §98 changed between 0.1.0-draft.1 and 1.1.0: a citation of it
# warns, naming both revisions. Exit 0: a warning is not a refusal.
plant "a cited section that changed" "WARN sas.section-current" \
    "sas://WAR-SAS-98 changed between SAS 0.1.0-draft.1.*SAS 1.1.0" 0 \
    "sed -i 's|$SEC_CITED|sas://WAR-SAS-98|' $SEC_AM; assert_present 'sas://WAR-SAS-98' $SEC_AM" \
    "$SEC_ALIAS"

# OBL-002 — the index is what `war compile` wrote, and a hand edit is drift.
if grep -q -- "\"revision\":\"1.1.0\"" docs/sas/generated/SECTIONS.json \
    && grep -q -- "\"sha256\":\"$SEC_DOC_SHA\"" docs/sas/generated/SECTIONS.json \
    && grep -q -- "sha256:$SEC_DOC_SHA (revision 1.1.0)" docs/sas/generated/SECTIONS.md; then
    sec_ok "the section index names its source" "sha256:${SEC_DOC_SHA:0:12}, revision 1.1.0"
else
    sec_fail "the section index names its source" "SECTIONS.* do not name the document and 1.1.0"
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

rm -rf "$SEC_TMP"
