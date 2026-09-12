# shellcheck shell=bash
# Token accounting (slice C2, SAS §33.7): the packet carries its estimate and
# budget, an over-budget stage is refused naming the largest items, and a
# budget that is not a number is a malformed graph.

TOK_TMP=$(mktemp -d)
MS=docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml

# The packet carries `tokens` (estimate, budget, method) inside its digest.
"$WAR" dispatch OW-WAR-0047 STAGE-002 --emit "$TOK_TMP/d.json" >/dev/null 2>&1
restore
if python3 - "$TOK_TMP/d.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); t = d.get("tokens") or {}
sys.exit(0 if t.get("method") == "oh.war/token-estimate/bytes-div-4/v1" and t.get("estimated_tokens", 0) > 0 and t.get("budget_tokens", 0) >= t["estimated_tokens"] else 1)
PY
then
    printf 'ok    %-34s estimate, budget and method are in the packet\n' "the packet carries its token account"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "the packet carries its token account" "$(python3 -c "import json; print(json.load(open('$TOK_TMP/d.json')).get('tokens'))" 2>&1)"
    FAILED=$((FAILED + 1))
fi

# A stage whose budget is smaller than its context is refused, and the refusal
# names what to cut.
plant_cmd "an over-budget stage is refused" "dispatch.over-budget" "largest:" 2 \
    "sed -i 's|^    executor_ref: \"materialize_dataset_path\"$|    executor_ref: \"materialize_dataset_path\"\n    budget_tokens: 10|' $MS; assert_present 'budget_tokens: 10' $MS" \
    dispatch OW-WAR-0047 STAGE-002 --emit "$TOK_TMP/x.json"

# A budget that is not a non-negative integer is a malformed milestones atom.
plant "a non-numeric budget is refused" "milestones.invalid" "budget_tokens" 2 \
    "sed -i 's|^    executor_ref: \"materialize_dataset_path\"$|    executor_ref: \"materialize_dataset_path\"\n    budget_tokens: \"lots\"|' $MS; assert_present 'budget_tokens: \"lots\"' $MS" \
    OW-WAR-0047

# The compile is journalled with its size.
"$WAR" dispatch OW-WAR-0047 STAGE-002 --emit "$TOK_TMP/d2.json" >/dev/null 2>&1
if tail -1 docs/warrants/OW-WAR-0047/journal.jsonl | grep -q '"type":"dispatch.compiled"' && tail -1 docs/warrants/OW-WAR-0047/journal.jsonl | grep -q 'estimated_tokens'; then
    printf 'ok    %-34s the last journal event is dispatch.compiled with its estimate\n' "a compile is journalled"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "a compile is journalled" "$(tail -1 docs/warrants/OW-WAR-0047/journal.jsonl | cut -c1-160)"
    FAILED=$((FAILED + 1))
fi
restore
rm -rf "$TOK_TMP"
