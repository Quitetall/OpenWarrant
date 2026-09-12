# shellcheck shell=bash
# Stage-relevant context selection (slice C1, SAS §47.2 / §33): a declared
# section is selected, a missing one is refused with the headings that exist,
# an unknown atom or artifact is refused, and a milestone may not carry a
# stage's context fields.

CTX_TMP=$(mktemp -d)
MS=docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml

# Positive: OW-WAR-0047 STAGE-002 (the corpus's dispatchable BLUT stage) declares two
# sections and two external refs;
# the emitted context manifest carries them, and the rest of the work order is
# not included whole.
"$WAR" dispatch OW-WAR-0047 STAGE-002 --emit "$CTX_TMP/d.json" --emit-context "$CTX_TMP/ctx.json" >/dev/null 2>&1
if python3 - "$CTX_TMP/ctx.json" <<'PY'
import json, sys
c = json.load(open(sys.argv[1]))
ids = [i["id"] for i in c["included"]]
wo = [i for i in c["included"] if i["id"].endswith("40-work-order.md")]
ok = len(wo) == 1 and sorted(wo[0].get("selector_sections", [])) == ["Deliverables", "Premade Instructions"] \
  and "sas://§47.2" in ids and "sas://§33" in ids \
  and all(not i.get("selector_sections") for i in c["included"] if not i["id"].endswith("40-work-order.md"))
sys.exit(0 if ok else 1)
PY
then
    printf 'ok    %-34s the work order carries its two selected sections; two external refs recorded unfetched\n' "a declared section is selected"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "a declared section is selected" "$(python3 -c "import json,sys; print([i['id'] for i in json.load(open('$CTX_TMP/ctx.json'))['included']])" 2>&1 | head -2)"
    FAILED=$((FAILED + 1))
fi

plant_cmd "a section that does not exist is refused" "dispatch.section-missing" "its headings are" 2 \
    "sed -i 's|40-work-order.md#Deliverables|40-work-order.md#Nope|' $MS; assert_present '#Nope' $MS" \
    dispatch OW-WAR-0047 STAGE-002 --emit "$CTX_TMP/x.json"

plant_cmd "an unknown atom is refused" "dispatch.unknown-atom" "99-nothing.md" 2 \
    "sed -i 's|^    context_external: .*$|    context_atoms: [\"99-nothing.md\"]|' $MS; assert_present '99-nothing.md' $MS" \
    dispatch OW-WAR-0047 STAGE-002 --emit "$CTX_TMP/x.json"

plant_cmd "a missing artifact is refused" "dispatch.artifact-missing" "no/such/file.txt" 2 \
    "sed -i 's|^    context_external: .*$|    context_artifacts: [\"no/such/file.txt\"]|' $MS; assert_present 'no/such/file.txt' $MS" \
    dispatch OW-WAR-0047 STAGE-002 --emit "$CTX_TMP/x.json"

# A milestone carrying a stage's context field is refused by the graph reader.
plant "a milestone may not carry context fields" "milestones.invalid" "context_sections" 2 \
    "sed -i '0,/^    stage_refs: \\[\"STAGE-001\"\\]/s//    stage_refs: [\"STAGE-001\"]\\n    context_sections: [\"40-work-order.md#Deliverables\"]/' $MS; assert_present 'context_sections: [\"40-work-order.md#Deliverables\"]' $MS" \
    OW-WAR-0047

rm -rf "$CTX_TMP"
