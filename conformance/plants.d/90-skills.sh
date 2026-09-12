# shellcheck shell=bash
# The skills over the core (OW-WAR-0068): `war frontier` derives four stage
# states from the records a resolution reads; the glossary rides in every
# Dispatch; an em-dash in an agent-facing document is refused by name.

# OW-WAR-0067's milestones: M1 open, M2 waits on M1, M3 waits on M2.
plant_cmd "an unblocked stage is OPEN" "OPEN" "OW-WAR-0067  STAGE-001" 0 \
    "true" frontier OW-WAR-0067
plant_cmd "a stage behind an incomplete milestone is blocked" "blocked" "waits on M1" 0 \
    "true" frontier OW-WAR-0067
plant_cmd "a dispatched stage is CLAIMED" "CLAIMED" "STAGE-001" 0 \
    "\"\$WAR\" dispatch OW-WAR-0068 STAGE-001 --emit /tmp/openwarrant-plant-dispatch.json >/dev/null 2>&1; assert_present 'dispatch.compiled' docs/warrants/OW-WAR-0068/journal.jsonl" \
    frontier OW-WAR-0068
rm -f /tmp/openwarrant-plant-dispatch.json
plant_cmd "frontier --json carries its schema" "oh.war/frontier/v1" "blocked" 0 \
    "true" --json frontier OW-WAR-0067

# The glossary is carried by the selector whenever it exists, and not invented when it does not.
SKILLS_TMP=$(mktemp -d)
"$WAR" dispatch OW-WAR-0047 STAGE-002 --emit-context "$SKILLS_TMP/with.json" >/dev/null 2>&1
if grep -q '"CONTEXT.md"' "$SKILLS_TMP/with.json"; then
    printf 'ok    %-34s CONTEXT.md is a selected item\n' "the glossary rides in the Dispatch"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s CONTEXT.md missing from the emitted context\n' "the glossary rides in the Dispatch"
    FAILED=$((FAILED + 1))
fi
restore
mv CONTEXT.md "$SKILLS_TMP/CONTEXT.md"
"$WAR" dispatch OW-WAR-0047 STAGE-002 --emit-context "$SKILLS_TMP/without.json" >/dev/null 2>&1
mv "$SKILLS_TMP/CONTEXT.md" CONTEXT.md
if ! grep -q '"CONTEXT.md"' "$SKILLS_TMP/without.json"; then
    printf 'ok    %-34s no glossary, no item\n' "an absent glossary is not invented"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s an item was emitted for a missing file\n' "an absent glossary is not invented"
    FAILED=$((FAILED + 1))
fi
restore

# writing-for-agents: an em-dash in a skill is refused by file and line.
printf '\nA planted sentence — with an em-dash.\n' >> .claude/skills/war-grill/SKILL.md
SKILLS_OUT=$(cargo xtask skills 2>&1); SKILLS_STATUS=$?
git checkout -- .claude/skills/war-grill/SKILL.md
if [[ $SKILLS_STATUS -ne 0 ]] && grep -q 'em-dash: .claude/skills/war-grill/SKILL.md' <<< "$SKILLS_OUT"; then
    printf 'ok    %-34s rejected by cargo xtask skills (file and line)\n' "an em-dash in a skill"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s\n%s\n' "an em-dash in a skill" "$SKILLS_STATUS" "$(head -3 <<< "$SKILLS_OUT")"
    FAILED=$((FAILED + 1))
fi
rm -rf "$SKILLS_TMP"
