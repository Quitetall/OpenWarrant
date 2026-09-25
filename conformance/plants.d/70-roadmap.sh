# shellcheck shell=bash
# OW-WAR-0114 — the roadmap is a record (OW-ADR-0023), and the checker holds
# Warrants to it. Every plant runs on a scratch program with a two-phase
# roadmap, so nothing here depends on this corpus's phases. OBL-004's last two
# checks read this corpus and write nothing to it.

echo "== roadmap (OW-ADR-0023) =="
PLANT_ROOT=$(scratch_corpus RM)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
RM_ALIAS=RM-WAR-0001
mkdir -p "$PLANT_ROOT/docs/roadmap/atoms"
cat > "$PLANT_ROOT/docs/roadmap/roadmap.toml" <<'TOML'
schema = "oh.war/roadmap/v1"
uuid = "01a0cd26-0000-7000-8000-000000000001"
program = "Plant Corpus RM"
prefix = "RM"

[[atoms]]
ordinal = 20
role = "phases"
path = "atoms/20-phases.yaml"
TOML
cat > "$PLANT_ROOT/docs/roadmap/atoms/20-phases.yaml" <<'YAML'
schema: "oh.war/roadmap-phases/v1"

phases:
  - id: "RM-PHASE-0"
    title: "Start"
    exit: "it starts"
    depends_on: []
  - id: "RM-PHASE-1"
    title: "Adopt"
    exit: "it is adopted"
    depends_on: ["RM-PHASE-0"]
YAML
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "roadmap" >/dev/null 2>&1
RM_MANIFEST="docs/warrants/$RM_ALIAS/manifest.toml"

# The scaffold's first Warrant cites RM-PHASE-1/adopt: a phase the roadmap has.
plant_cmd "a ref to a declared phase passes" "roadmap.valid" "2 phases" 0 ":" check

# A ref to a phase the roadmap lacks is refused by name.
plant "a ref to an undeclared phase" "roadmap.unknown-phase" "RM-PHASE-7" 2 \
    "sed -i 's|roadmap://RM-PHASE-1/adopt|roadmap://RM-PHASE-7/adopt|' $RM_MANIFEST; assert_present 'RM-PHASE-7' '$RM_MANIFEST'"

# A dependency cycle is refused as a cycle, not as "malformed".
plant "a phase dependency cycle" "roadmap.cycle" "cycle" 2 \
    "sed -i 's|depends_on: \[\]|depends_on: [\"RM-PHASE-1\"]|' docs/roadmap/atoms/20-phases.yaml; assert_present 'depends_on: [\"RM-PHASE-1\"]' docs/roadmap/atoms/20-phases.yaml"

# An unsigned Warrant with no ref warns, and `assign` gives it one.
plant_restore
sed -i '/^\[\[roadmap\]\]$/,/^ref = /d' "$PLANT_ROOT/$RM_MANIFEST"
UA=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
AS=$("$WAR" --root "$PLANT_ROOT" roadmap assign "$RM_ALIAS" RM-PHASE-1/adopt 2>&1)
UA2=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
plant_restore
if grep -q "roadmap.unassigned .*$RM_ALIAS" <<<"$UA" && grep -q 'roadmap.assigned' <<<"$AS" \
    && ! grep -q 'roadmap.unassigned' <<<"$UA2"; then
    printf 'ok    %-34s warned, then assigned\n' "an unassigned draft is assignable"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no warning, or assign did not clear it\n' "an unassigned draft is assignable"
    FAILED=$((FAILED + 1))
fi

# The atoms as they stand are not accepted: a warning, and `propose` records
# the revision that a human accepts in one act.
PR_BEFORE=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
PR=$("$WAR" --root "$PLANT_ROOT" roadmap propose 2>&1)
PR_AFTER=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
plant_restore
rm -rf "$PLANT_ROOT/docs/roadmap/revisions"
if grep -q 'roadmap.unaccepted .*not an accepted revision' <<<"$PR_BEFORE" \
    && grep -q 'revision 1 proposed' <<<"$PR" \
    && grep -q 'roadmap.unaccepted .*awaits one signature' <<<"$PR_AFTER"; then
    printf 'ok    %-34s unaccepted → proposed rev 1 → awaits one signature\n' "roadmap revisions are proposed"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the unaccepted → proposed path did not read as expected\n' "roadmap revisions are proposed"
    FAILED=$((FAILED + 1))
fi

# ---------------------------------------------------------------- OBL-004 --
# The master document carries the roadmap, earlier revisions go to the
# history, and a retired plan is one line of lineage. Each claim is paired
# with the refusal that shows it is computed, not written.
rm_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rm_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
rm_commit() { git -C "$PLANT_ROOT" add -A >/dev/null 2>&1; git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1; }

plant_restore
rm -rf "$PLANT_ROOT/docs/roadmap/revisions"
if grep -q '^history = ' "$PLANT_ROOT/openwarrant.toml"; then
    sed -i 's/^history = .*$/history = true/' "$PLANT_ROOT/openwarrant.toml"
else
    sed -i 's/^verify_drift = true$/verify_drift = true\nhistory = true/' "$PLANT_ROOT/openwarrant.toml"
fi
grep -q '^history = true' "$PLANT_ROOT/openwarrant.toml" || { printf 'PLANT SETUP FAILED: history key not set\n' >&2; exit 9; }
# Revision 1 is the two phases; revision 2 adds the lineage of an older plan.
"$WAR" --root "$PLANT_ROOT" roadmap propose >/dev/null 2>&1
printf 'the plan before the record\n' > "$PLANT_ROOT/docs/old-plan.md"
cat >> "$PLANT_ROOT/docs/roadmap/roadmap.toml" <<'TOML'

[[retires]]
path = "docs/old-plan.md"
lineage = "the plant's earlier plan, retired: its phases are this record's."
TOML
RM_PR2=$("$WAR" --root "$PLANT_ROOT" roadmap propose 2>&1)
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
rm_commit "two roadmap revisions, one retired plan, compiled"
[[ -f "$PLANT_ROOT/docs/roadmap/revisions/2.toml" ]] || { printf 'PLANT SETUP FAILED: revision 2 not proposed\n%s\n' "$RM_PR2" >&2; exit 9; }

RM_CUR="$PLANT_ROOT/docs/generated/CURRENT.md"
RM_HIS="$PLANT_ROOT/docs/generated/HISTORY.md"
RM_SEC=$(awk '/^## Roadmap$/{f=1; next} /^## /{f=0} f' "$RM_CUR")
RM_P0=$(grep -n '^#### RM-PHASE-0 — Start$' <<<"$RM_SEC" | cut -d: -f1)
RM_P1=$(grep -n '^#### RM-PHASE-1 — Adopt$' <<<"$RM_SEC" | cut -d: -f1)
if [[ -n "$RM_P0" && -n "$RM_P1" && "$RM_P0" -lt "$RM_P1" ]] \
    && grep -qxF -- '- **Exit:** it is adopted' <<<"$RM_SEC" \
    && grep -qE -- "^- \*\*Members \(1\):\*\* $RM_ALIAS \`[a-z_]+\`\$" <<<"$RM_SEC" \
    && grep -qF -- '**after:** RM-PHASE-0' <<<"$RM_SEC" \
    && grep -qE -- '^- \*\*Achieved:\*\* ' <<<"$RM_SEC"; then
    rm_ok "CURRENT.md carries the roadmap" "phases in order; exit, members with rungs, achievement"
else
    rm_fail "CURRENT.md carries the roadmap" "$(head -c 400 <<<"$RM_SEC" | tr '\n' '|')"
fi
# The queue row for revision 2 names its diff; outside table rows the plan
# is named exactly once.
if [[ $(grep -v '^|' "$RM_CUR" | grep -c 'docs/old-plan.md') -eq 1 ]] \
    && grep -qxF -- "- \`docs/old-plan.md\` → the plant's earlier plan, retired: its phases are this record's." <<<"$RM_SEC"; then
    rm_ok "a retired plan is one line" "named once in CURRENT.md, as lineage"
else
    rm_fail "a retired plan is one line" "$(grep -n 'old-plan' "$RM_CUR" | head -3 | tr '\n' '|')"
fi
RM_H1=$(grep -n '^### Revision 1 — proposed$' "$RM_HIS" | cut -d: -f1)
RM_H2=$(grep -n '^### Revision 2 — proposed$' "$RM_HIS" | cut -d: -f1)
if [[ -n "$RM_H1" && -n "$RM_H2" && "$RM_H1" -lt "$RM_H2" ]] \
    && grep -qE -- '^- \*\*Diff against its predecessor:\*\* .*retires docs/old-plan.md$' "$RM_HIS" \
    && grep -qF -- '1 earlier revision(s) are in the history.' <<<"$RM_SEC" \
    && ! grep -q '^### Revision ' "$RM_CUR"; then
    rm_ok "the history holds earlier revisions" "revisions 1 and 2 in HISTORY.md; CURRENT.md names the count"
else
    rm_fail "the history holds earlier revisions" "$(grep -n 'Revision' "$RM_HIS" "$RM_CUR" | head -4 | tr '\n' '|')"
fi

# Membership and achievement are computed: a hand edit to the section is
# drift, and a phase that writes its members is refused by name.
plant "a hand edit to the Roadmap section" "generated.drift" "CURRENT.md" 2 \
    "sed -i 's/^- \*\*Achieved:\*\* .*/- **Achieved:** achieved (claimed by hand)/' docs/generated/CURRENT.md; assert_present 'claimed by hand' docs/generated/CURRENT.md" \
    --generated
plant "a phase that lists its members" "roadmap.malformed" "RM-PHASE-1 writes .members." 2 \
    "sed -i 's/^    title: \"Adopt\"$/    title: \"Adopt\"\n    members: [\"$RM_ALIAS\"]/' docs/roadmap/atoms/20-phases.yaml; assert_present 'members:' docs/roadmap/atoms/20-phases.yaml"
plant "a plan retired twice" "roadmap.malformed" "is named twice" 2 \
    "printf '\n[[retires]]\npath = \"docs/old-plan.md\"\nlineage = \"again\"\n' >> docs/roadmap/roadmap.toml; assert_present 'lineage = \"again\"' docs/roadmap/roadmap.toml"
plant "a lineage of two lines" "roadmap.malformed" "exactly one non-empty line" 2 \
    "printf '\n[[retires]]\npath = \"docs/other-plan.md\"\nlineage = \"\"\"one\ntwo\"\"\"\n' >> docs/roadmap/roadmap.toml; assert_present 'docs/other-plan.md' docs/roadmap/roadmap.toml"

# `war progress` builds its tree from the record with view.json absent; with
# the record gone too, there is no tree — nothing else supplies one.
RM_PG=$("$WAR" --root "$PLANT_ROOT" progress --snapshot --json 2>/dev/null)
mv "$PLANT_ROOT/docs/roadmap/roadmap.toml" "$PLANT_ROOT/docs/roadmap/roadmap.toml.off"
RM_PG_NONE=$("$WAR" --root "$PLANT_ROOT" progress --snapshot --json 2>/dev/null)
plant_restore
if [[ ! -e "$PLANT_ROOT/docs/roadmap/view.json" ]] \
    && python3 -c 'import json,sys; r=json.loads(sys.argv[1])["result"]["roadmap"]; ids=[n["id"] for n in r["nodes"]]; assert "RM-PHASE-0" in ids and "RM-PHASE-1" in ids, ids' "$RM_PG" 2>/dev/null \
    && python3 -c 'import json,sys; assert json.loads(sys.argv[1])["result"]["roadmap"] is None' "$RM_PG_NONE" 2>/dev/null; then
    rm_ok "war progress reads the record" "no view.json: phases from the record; no record: no tree"
else
    rm_fail "war progress reads the record" "$(head -c 300 <<<"$RM_PG" | tr '\n' ' ')"
fi

# On this corpus (read only): the four legacy plans are lineage, once each;
# the three that could be are one line on disk, and the rc.3 inventory its
# draft pins is untouched.
RM_REPO_SEC=$(awk '/^## Roadmap$/{f=1; next} /^## /{f=0} f' docs/generated/CURRENT.md)
RM_LEGACY_OK=1
for f in docs/roadmap/PRODUCTION_ROADMAP.md docs/roadmap/view.json docs/design/rc2-implementation-roadmap.json docs/sas/drafts/1.0.0-rc.3/roadmap.json; do
    [[ $(grep -cF -- "- \`$f\` → " <<<"$RM_REPO_SEC") -eq 1 ]] || RM_LEGACY_OK=0
done
for f in docs/roadmap/PRODUCTION_ROADMAP.md docs/roadmap/view.json docs/design/rc2-implementation-roadmap.json; do
    [[ $(wc -l < "$f") -eq 1 ]] && grep -q 'Retired by OW-WAR-0114' "$f" || RM_LEGACY_OK=0
done
(cd docs/sas/drafts/1.0.0-rc.3 && python3 check_draft.py >/dev/null 2>&1) || RM_LEGACY_OK=0
if [[ "$RM_LEGACY_OK" -eq 1 ]]; then
    rm_ok "the legacy roadmaps are lineage" "four lines in CURRENT.md; three files one line; rc.3 draft PASS"
else
    rm_fail "the legacy roadmaps are lineage" "a lineage line is missing or doubled, a file is not retired, or the rc.3 draft check fails"
fi

# The SAS revision that points §98 at the record (1.1.1, OW-WAR-0114 M5) is
# recorded, the document's §98 is that pointer with no phase listed, and the
# document matches a recorded revision — which one, and in which state, is
# derived: it moved from "1.1.1 proposed" to "1.1.1 accepted" to a later
# proposal as the owner acted, and a check written against one moment went
# stale at the next.
RM_SAS=$("$WAR" sas status 2>&1)
RM_POINTER_STATE=$(sed -n 's/^state = "\(.*\)"/\1/p' docs/sas/revisions/1.1.1.toml 2>/dev/null)
if [[ "$RM_POINTER_STATE" == accepted || "$RM_POINTER_STATE" == proposed ]] \
    && grep -qE 'PASS sas.revision +[0-9][^ ]* · (accepted|proposed) · .* matches the document' <<<"$RM_SAS" \
    && awk '/^## 98\. /{f=1; next} /^## /{f=0} f' docs/sas/WAR_Software_Architecture_Specification.md | grep -qxF "The phases are the roadmap record's (OW-ADR-0023)." \
    && ! grep -q '^### Phase 0 ' docs/sas/WAR_Software_Architecture_Specification.md; then
    rm_ok "§98 points at the record" "1.1.1 $RM_POINTER_STATE; the document matches a recorded revision"
else
    rm_fail "§98 points at the record" "1.1.1 ${RM_POINTER_STATE:-unrecorded}: $(grep -E 'sas.revision' <<<"$RM_SAS" | tail -2 | tr '\n' '|')"
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT

# On this repository, after the scratch program is gone (PLANT_ROOT unset):
# the refusal: a document edited after the proposal matches no revision, and
# the check says so instead of reading the proposal as still in hand.
RM_LATEST=$(grep -l '^state = "accepted"' docs/sas/revisions/*.toml | xargs -n1 basename | sed 's/\.toml$//' | sort -V | tail -1)
plant "the SAS edited after its proposal" "sas.digest-drift" "revision $RM_LATEST (accepted) records" 2 \
    "printf '\nThe phases are listed here after all.\n' >> docs/sas/WAR_Software_Architecture_Specification.md; assert_present 'listed here after all' docs/sas/WAR_Software_Architecture_Specification.md"
