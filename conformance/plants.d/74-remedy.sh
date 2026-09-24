# shellcheck shell=bash
# OW-WAR-0112 — every diagnostic carries its remedy (§76.2).
#
# One `war check` in each mode over a planted drift, then questions of its
# output. The envelope goes to python on stdin — as one argument it is past
# the kernel's 128 KiB limit and the plant would score an empty string.
#
# The drift is planted, not found: the corpus was red with relicense drift
# when this was written and the plant read that, so it failed the day the
# owner signed the corrections (2026-09-23). A byte appended to a file a
# resolved Warrant pins (OW-WAR-0061's D-001, under a path `restore` puts
# back) is the drift, and it is taken out again before anything else runs.

echo "== remedies (§76.2) =="
RM_FILE=docs/roadmap/PHASE1_EXIT.md
printf '\n<!-- planted drift -->\n' >> "$RM_FILE"
assert_present 'planted drift' "$RM_FILE"
RM_JSON=$("$WAR" check --json 2>/dev/null)
RM_HUMAN=$("$WAR" check 2>&1)
git checkout -- "$RM_FILE"

# A drift diagnostic carries a HUMAN remedy naming the correction act with
# the alias and deliverable filled in — never placeholders.
if python3 -c '
import sys, json
d = json.load(sys.stdin)
drift = [x for x in d["diagnostics"] if x["rule"] == "deliverable.digest-drift"]
assert drift, "no drift on this corpus to look at"
for x in drift:
    r = x["remedy"]
    assert r["kind"] == "human", r
    assert r["argv"][:2] == ["war", "correct"], r
    assert "<" not in " ".join(r["argv"]), r
' <<<"$RM_JSON" 2>/dev/null; then
    printf 'ok    %-34s human, war correct <alias> <D>, no placeholders\n' "drift carries a human correction"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s remedy missing, wrong kind, or placeholders left in\n' "drift carries a human correction"
    FAILED=$((FAILED + 1))
fi

# No AUTO remedy is a signing act: `sign`, `authorize`, `resolve`, `correct`,
# `accept`, `answer` — the invariant `next.rs` holds for agent actions.
if python3 -c '
import sys, json
d = json.load(sys.stdin)
verbs = {"sign", "authorize", "resolve", "correct", "accept", "answer"}
bad = [x["rule"] for x in d["diagnostics"]
       if x.get("remedy") and x["remedy"]["kind"] == "auto"
       and x["remedy"]["argv"][0] == "war" and x["remedy"]["argv"][1] in verbs]
assert not bad, bad
autos = [x for x in d["diagnostics"] if x.get("remedy") and x["remedy"]["kind"] == "auto"]
assert autos, "no auto remedy on this corpus at all"
' <<<"$RM_JSON" 2>/dev/null; then
    printf 'ok    %-34s no auto remedy signs\n' "auto remedies never sign"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s an auto remedy carries a signing verb\n' "auto remedies never sign"
    FAILED=$((FAILED + 1))
fi

# A PASS carries no remedy; the field is absent, not null (additive schema).
if python3 -c '
import sys, json
d = json.load(sys.stdin)
assert all("remedy" not in x for x in d["diagnostics"] if x["severity"] == "pass")
assert all(x.get("remedy", {}) is not None for x in d["diagnostics"])
' <<<"$RM_JSON" 2>/dev/null; then
    printf 'ok    %-34s absent on a pass, never null\n' "a pass has no remedy"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s a pass carries a remedy, or a remedy is null\n' "a pass has no remedy"
    FAILED=$((FAILED + 1))
fi

# The human rendering: a third line under the finding, and one REMEDIES
# block with counts before the totals.
if grep -q '→ human: war correct OW-WAR-' <<<"$RM_HUMAN" \
    && [[ $(grep -c '^REMEDIES:' <<<"$RM_HUMAN") -eq 1 ]] \
    && grep -qE '^  (auto|human|info) +war .*\(×[0-9]+\)$' <<<"$RM_HUMAN"; then
    printf 'ok    %-34s third line + one REMEDIES block\n' "remedies render for a human"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no remedy line, or REMEDIES block missing/duplicated\n' "remedies render for a human"
    FAILED=$((FAILED + 1))
fi

# The envelope's top-level keys are still the frozen eight (§76.4); the
# remedy is INSIDE a diagnostic, additive.
if python3 -c '
import sys, json
d = json.load(sys.stdin)
assert sorted(d.keys()) == ["command", "counts", "diagnostics", "exit_code", "notes", "schema", "verdict", "verdict_line"], sorted(d.keys())
' <<<"$RM_JSON" 2>/dev/null; then
    printf 'ok    %-34s eight keys, remedy inside the diagnostic\n' "envelope keys unchanged"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the envelope grew a top-level key\n' "envelope keys unchanged"
    FAILED=$((FAILED + 1))
fi
