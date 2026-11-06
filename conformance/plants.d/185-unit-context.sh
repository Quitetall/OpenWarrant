#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# oh.war/unit-context/v1: what `war context` gives when a record packet would
# refuse. Each answer is beside the input that must still refuse:
#   1. a long document gives what fits and declares its cuts; --strict refuses;
#   2. a unit is named; a unit that does not exist is refused with what does;
#   3. words find a unit; words nothing matches are refused;
#   4. an id that names nothing is refused, whatever its words match;
#   5. an asked-for budget still refuses (section 10.3);
#   6. an edit is seen at once, through the graph cache.

echo "== unit context: long documents, topics and paths =="
PLANT_ROOT=$(scratch_corpus UC)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
UC_TMP=$(mktemp -d)
uc_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
uc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ucw() { "$WAR" --root "$PLANT_ROOT" "$@"; }

python3 - "$PLANT_ROOT/docs/UC_COMBAT.md" <<'PY'
import sys
filler = "Armies march on stomachs and spreadsheets.\n" * 50
doc = "# Combat\n\nHow fights resolve.\n\n"
doc += "".join(f"## Rule {i}\n\n{filler}\n" for i in range(20))
doc += "## Friendly fire {#ff}\n\nShots at allies hit at half damage.\n"
open(sys.argv[1], "w", encoding="utf-8").write(doc)
PY

ucw context docs/UC_COMBAT.md > "$UC_TMP/doc.md" 2>/dev/null; UC_RC=$?
ucw context docs/UC_COMBAT.md --strict > "$UC_TMP/strict.md" 2>"$UC_TMP/strict.err"; UC_STRICT=$?
if [[ $UC_RC -eq 0 && $(wc -c < "$UC_TMP/doc.md") -le 16000 ]] && grep -q '^## Cut' "$UC_TMP/doc.md" \
    && grep -q '^\[S\] ' "$UC_TMP/doc.md" && [[ $UC_STRICT -eq 2 && ! -s "$UC_TMP/strict.md" ]] \
    && grep -q 'context.over-budget' "$UC_TMP/strict.err"; then
    uc_ok "a long document degrades" "$(wc -c < "$UC_TMP/doc.md") bytes, cuts declared; --strict refused"
else
    uc_fail "a long document degrades" "exit $UC_RC, $(wc -c < "$UC_TMP/doc.md") bytes; strict exit $UC_STRICT"
fi

ucw context 'docs/UC_COMBAT.md#ff' > "$UC_TMP/unit.md" 2>/dev/null; UC_UNIT=$?
ucw context 'docs/UC_COMBAT.md#nope' > "$UC_TMP/none.md" 2>"$UC_TMP/none.err"; UC_NONE=$?
if [[ $UC_UNIT -eq 0 ]] && grep -q 'half damage' "$UC_TMP/unit.md" && ! grep -q 'Armies' "$UC_TMP/unit.md" \
    && [[ $UC_NONE -eq 2 && ! -s "$UC_TMP/none.md" ]] && grep -q 'rule-0' "$UC_TMP/none.err"; then
    uc_ok "a unit is selected by id" "one unit returned; a missing one refused, naming what exists"
else
    uc_fail "a unit is selected by id" "unit exit $UC_UNIT; missing exit $UC_NONE"
fi

ucw context 'friendly fire' > "$UC_TMP/topic.md" 2>/dev/null; UC_TOPIC=$?
ucw context 'quantum zebra' > "$UC_TMP/zebra.md" 2>"$UC_TMP/zebra.err"; UC_ZEBRA=$?
if [[ $UC_TOPIC -eq 0 ]] && grep -q '^\[F\] md:docs/UC_COMBAT.md#ff' "$UC_TMP/topic.md" \
    && [[ $UC_ZEBRA -eq 2 && ! -s "$UC_TMP/zebra.md" ]] && grep -q 'context.target-unknown' "$UC_TMP/zebra.err"; then
    uc_ok "a topic finds its unit" "ranked and in full; unmatched words refused"
else
    uc_fail "a topic finds its unit" "topic exit $UC_TOPIC; unmatched exit $UC_ZEBRA"
fi

ucw context 'UC-WAR-9999' > "$UC_TMP/id.md" 2>"$UC_TMP/id.err"; UC_ID=$?
if [[ $UC_ID -eq 2 && ! -s "$UC_TMP/id.md" ]] && grep -q 'context.target-unknown' "$UC_TMP/id.err"; then
    uc_ok "an unknown id stays refused" "no search stands in for a failed lookup"
else
    uc_fail "an unknown id stays refused" "exit $UC_ID"
fi

ucw context docs/UC_COMBAT.md --budget 100 > "$UC_TMP/tiny.md" 2>"$UC_TMP/tiny.err"; UC_TINY=$?
if [[ $UC_TINY -eq 2 && ! -s "$UC_TMP/tiny.md" ]] && grep -q 'context.over-budget' "$UC_TMP/tiny.err"; then
    uc_ok "an asked-for budget refuses" "section 10.3 stands for an explicit budget"
else
    uc_fail "an asked-for budget refuses" "exit $UC_TINY"
fi

sed -i 's/half damage/no damage/' "$PLANT_ROOT/docs/UC_COMBAT.md"
UC_AFTER=$(ucw context 'docs/UC_COMBAT.md#ff' 2>/dev/null)
if grep -q 'no damage' <<<"$UC_AFTER"; then
    uc_ok "an edit is seen at once" "the cached graph never answers for a changed tree"
else
    uc_fail "an edit is seen at once" "$(head -3 <<<"$UC_AFTER" | tr '\n' '|')"
fi

/usr/bin/rm -rf "$UC_TMP"
unset UC_TMP UC_RC UC_STRICT UC_UNIT UC_NONE UC_TOPIC UC_ZEBRA UC_ID UC_TINY UC_AFTER
