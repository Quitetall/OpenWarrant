# shellcheck shell=bash
# OW-WAR-0114 — the roadmap is a record (OW-ADR-0023), and the checker holds
# Warrants to it. Every plant runs on a scratch program with a two-phase
# roadmap, so nothing here depends on this corpus's phases.

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

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
