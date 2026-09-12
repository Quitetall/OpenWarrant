# shellcheck shell=bash
# `war plan` — the drafting seam with something on the other side (ADR-0013).
# Fixture-driven (OW-WAR-0042 OBS-002): every plant feeds a committed file, so
# a plant whose mutation was a no-op cannot exercise the happy path unnoticed.

FX=conformance/fixtures

# §74.4 step 6 — review is a recorded step; --apply without it is refused.
plant_cmd "apply without review is refused" "74.4" "semantic diff" 1 \
    "true" \
    plan --proposal "$FX/proposals/v2-minimal.json" --apply

# A v1 proposal validates and cannot be applied: its operations have no payload.
plant_cmd "a v1 proposal cannot be applied" "plan.v1-has-no-payloads" "no payload" 1 \
    "true" \
    plan --proposal "$FX/proposals/v1-bare.json" --reviewed --apply

# §91.8 test 58 — an unanswered blocker stops a non-interactive run.
plant_cmd "an unanswered blocker stops the run" "plan.interview-required" "Q-001" 1 \
    "true" \
    plan --proposal "$FX/proposals/v2-blocker.json" --reviewed

# …and an answer clears it (positive).
plant_cmd "an answered blocker proceeds" "plan.applicable" "applicable" 0 \
    "true" \
    plan --proposal "$FX/proposals/v2-blocker.json" --reviewed --answer "Q-001=keep a changelog"

# §74.3 — an operation outside the closed list is refused at parse.
plant_cmd "write_file is not an operation" "did not parse" "write_file" 1 \
    "true" \
    plan --proposal "$FX/proposals/v2-write-file.json" --reviewed


# The committed openwarrant.toml names a real drafter (OW-WAR-0042), so a
# plant that wants a fixture drafter REPLACES the `[plan]` table instead of
# appending a second one (a duplicate table is a TOML error, not a plant).
plan_clear_drafter() {
    python3 - <<'PY'
import pathlib, re
p = pathlib.Path("openwarrant.toml"); s = p.read_text()
s = re.sub(r'\n\[plan\]\n(?:(?!\[).*\n?)*', '\n', s)
p.write_text(s.rstrip("\n") + "\n")
PY
}
plan_set_drafter() {
    plan_clear_drafter
    printf '\n[plan]\n%s\n' "$1" >> openwarrant.toml
}

# No drafter configured: --draft says so rather than inventing one.
plant_cmd "no drafter configured" "no drafter is configured" "drafter_argv" 1 \
    "plan_clear_drafter; assert_gone 'drafter_argv' openwarrant.toml" \
    plan "add a changelog" --draft

# §74.5 — a drafter that touches the tree is refused and its proposal discarded.
plant_cmd "a drafter that writes a file is refused" "plan.drafter-wrote-files" "README.planted.md" 1 \
    "plan_set_drafter 'drafter_argv = [\"bash\", \"$FX/drafter/writes-a-file.sh\"]'; \
     assert_present 'writes-a-file.sh' openwarrant.toml" \
    plan "add a changelog" --draft
rm -f README.planted.md

# A drafter that never answers is killed at the configured bound.
plant_cmd "a drafter that never answers is killed" "plan.drafter-timeout" "killed" 1 \
    "plan_set_drafter 'drafter_argv = [\"bash\", \"$FX/drafter/sleeps.sh\"]
drafter_timeout_secs = 1'; \
     assert_present 'sleeps.sh' openwarrant.toml" \
    plan "add a changelog" --draft

# Positive, end to end: the configured drafter answers, review is recorded, and
# --apply creates a Warrant through the seven operations. The new directory is
# untracked, so it is removed here rather than by `restore`.
plan_set_drafter "drafter_argv = [\"bash\", \"$FX/drafter/echo-proposal.sh\"]
drafter_name = \"echo-fixture\""
APPLY_OUT=$("$WAR" --json plan "add a changelog" --draft --reviewed --apply --out /tmp/openwarrant-plant-proposal.json 2>/dev/null)
APPLY_STATUS=$?
NEW_ALIAS=$(printf '%s' "$APPLY_OUT" | python3 -c 'import sys, json
try:
    v = json.load(sys.stdin); print(v["result"]["alias"])
except Exception:
    print("")')
# The directory `war new` allocated, found from the tree rather than from the
# report, so a failed apply is still cleaned up (and named in the FAIL line).
NEW_DIR=$(git status --porcelain --untracked-files=normal -- docs/warrants | sed -n 's|^?? docs/warrants/\(OW-WAR-[0-9]*\)/$|\1|p' | head -1)
git checkout -- openwarrant.toml
if [[ "$APPLY_STATUS" -eq 0 && -n "$NEW_ALIAS" && -f "docs/warrants/$NEW_ALIAS/plan/proposal.json" && -f "docs/warrants/$NEW_ALIAS/plan/drafter.json" ]] \
   && grep -q 'plan.applied' "docs/warrants/$NEW_ALIAS/journal.jsonl"; then
    printf 'ok    %-34s %s created through the seam with its provenance recorded\n' "a drafted proposal is applied" "$NEW_ALIAS"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s alias %s dir %s\n' "a drafted proposal is applied" "$APPLY_STATUS" "${NEW_ALIAS:-none}" "${NEW_DIR:-none}"
    FAILED=$((FAILED + 1))
fi
[[ -n "$NEW_DIR" ]] && rm -rf "docs/warrants/$NEW_DIR"
rm -f /tmp/openwarrant-plant-proposal.json
git checkout -- docs/adr/ docs/warrants/generated 2>/dev/null || true

# §91.8 test 57 — validating the same proposal twice yields byte-identical
# output. Under --json the envelope is the whole answer, so cmp is the test.
PLAN57_TMP=$(mktemp -d)
sed 's/{{NS}}/OW/g' conformance/fixtures/proposals/v2-minimal.json > "$PLAN57_TMP/p.json" 2>/dev/null || cp conformance/fixtures/proposals/v2-minimal.json "$PLAN57_TMP/p.json"
"$WAR" --json plan --proposal "$PLAN57_TMP/p.json" > "$PLAN57_TMP/a.json" 2>/dev/null
"$WAR" --json plan --proposal "$PLAN57_TMP/p.json" > "$PLAN57_TMP/b.json" 2>/dev/null
if [[ -s "$PLAN57_TMP/a.json" ]] && cmp -s "$PLAN57_TMP/a.json" "$PLAN57_TMP/b.json" && grep -Eq '"plan\.(not-)?applicable"' "$PLAN57_TMP/a.json"; then
    printf 'ok    %-34s two validations, one envelope (§91.8 test 57)\n' "validation output is byte-identical"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the two validation envelopes differ or are empty\n' "validation output is byte-identical"
    FAILED=$((FAILED + 1))
fi
rm -rf "$PLAN57_TMP"
