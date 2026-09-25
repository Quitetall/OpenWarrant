# shellcheck shell=bash
# t-dc28 — amendment ids two branches cannot both mint (§31 records).
#
# `war amend` names a new amendment `AM-<n>-<hash>`: two branches that amend
# the same Warrant from the same base write two files, and the merge has
# nothing to resolve. The records written before — `AM-<n>` — keep their
# names, their bytes and their order; the signed contracts that cite them do
# not move. A name that is not an amendment id, or a record whose `id:` is not
# its name, is refused by name (`amendment.id`). Order: ordinal, then
# `effective_time`, then name (crates/openwarrant-cli/src/amendment_id.rs).
#
# The branches are real git branches of a scratch program; the live corpus is
# only read, except by one refused `war amend` that must write nothing.

echo "== amendment ids (t-dc28) =="
PLANT_ROOT=$(scratch_corpus AI)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
ai_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ai_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
aiw() { env -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
ai_git() { git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
AI_ALIAS=$(ls "$PLANT_ROOT/docs/warrants" | grep -E '^AI-WAR-[0-9]{4}$' | head -1)
[[ -n "$AI_ALIAS" ]] || { printf 'PLANT SETUP FAILED: the scratch program has no Warrant\n' >&2; exit 9; }
AI_DIR="$PLANT_ROOT/docs/warrants/$AI_ALIAS/amendments"

# ai_fill <file> <effective_time>: write what `war amend` leaves to a person.
ai_fill() {
    python3 - "$1" "$2" <<'PY'
import pathlib, re, sys
p = pathlib.Path(sys.argv[1]); s = p.read_text()
for k, v in [("reason", "A branch amends the work order."),
             ("governing_adr_or_policy", "sas://WAR-SAS-31"),
             ("restart_or_repair_instruction", "Continue."),
             ("authorizer", "plant"),
             ("effective_time", sys.argv[2])]:
    s = re.sub(rf'(?m)^{k}: .*$', f'{k}: "{v}"', s)
s = s.replace("semantic_diff: []",
              'semantic_diff:\n  - element: "deliverables"\n    before: "a"\n    after: "b"')
p.write_text(s)
PY
}
# ai_amend -> the stem `war amend` wrote, read from its own report.
ai_amend() {
    aiw amend "$AI_ALIAS" 2>&1 | grep -oE 'amendments/AM-[0-9]+-[0-9a-f]+\.yaml' | head -1 | sed -E 's|amendments/||; s|\.yaml$||'
}

# The base both branches start from: one amendment, written the old way.
mkdir -p "$AI_DIR"
cat > "$AI_DIR/AM-001.yaml" <<'YAML'
schema: "oh.war/amendment/v1"

id: "AM-001"
band: "manual_revision"
reason: "Written before t-dc28, named the old way."
governing_adr_or_policy: "sas://WAR-SAS-31"
artifact_admissibility: "remain_admissible"
restart_or_repair_instruction: "Continue."
re_preflight_required: "false"
authorizer: "plant"
effective_time: "2026-09-20"

semantic_diff:
  - element: "deliverables"
    before: "x"
    after: "y"

affected_stages: []
affected_milestones: []
YAML
ai_git add -A >/dev/null 2>&1 && ai_git commit -qm "a legacy AM-001" >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: base commit in %s\n' "$PLANT_ROOT" >&2; exit 9; }
AI_BASE=$(git -C "$PLANT_ROOT" rev-parse HEAD)

# ------------------------------------------------------------------ accept --
# Two branches, each unaware of the other, amend the same Warrant.
ai_git checkout -q -b ai-a "$AI_BASE"
AI_A=$(ai_amend)
[[ -n "$AI_A" ]] && ai_fill "$AI_DIR/$AI_A.yaml" "2026-09-25T10:00:00Z"
ai_git add -A >/dev/null 2>&1; ai_git commit -qm "branch a amends" >/dev/null 2>&1
ai_git checkout -q -b ai-b "$AI_BASE"
AI_B=$(ai_amend)
[[ -n "$AI_B" ]] && ai_fill "$AI_DIR/$AI_B.yaml" "2026-09-25T09:00:00Z"
ai_git add -A >/dev/null 2>&1; ai_git commit -qm "branch b amends" >/dev/null 2>&1
ai_git merge -q --no-edit ai-a >/dev/null 2>&1; AI_MERGE=$?
AI_CHK=$(aiw check "$AI_ALIAS" 2>&1); AI_CHK_S=$?
AI_ORDER=$(aiw status --json 2>/dev/null | python3 -c "
import json, sys
v = json.load(sys.stdin)
w = [w for w in v['result']['warrants'] if w['alias'] == sys.argv[1]][0]
print(','.join(a['id'] for a in w.get('amendments', [])))" "$AI_ALIAS" 2>/dev/null)
if [[ "$AI_A" =~ ^AM-002-[0-9a-f]{4,16}$ && "$AI_B" =~ ^AM-002-[0-9a-f]{4,16}$ && "$AI_A" != "$AI_B" \
    && $AI_MERGE -eq 0 && $AI_CHK_S -eq 0 && "$AI_ORDER" == "AM-001,$AI_B,$AI_A" ]] \
    && grep -q "amendment.valid .*amendment $AI_A " <<<"$AI_CHK" \
    && grep -q "amendment.valid .*amendment $AI_B " <<<"$AI_CHK" \
    && ! grep -q 'amendment.id' <<<"$AI_CHK"; then
    ai_ok "two branches, two amendment files" "$AI_A and $AI_B merge clean; order AM-001, $AI_B (09:00), $AI_A (10:00)"
else
    ai_fail "two branches, two amendment files" "a=$AI_A b=$AI_B merge=$AI_MERGE check=$AI_CHK_S order=$AI_ORDER"
fi
# The next one counts past both.
AI_NEXT=$(aiw amend "$AI_ALIAS" --dry-run 2>&1)
if grep -qE "amend.would-write .*amendments/AM-003-[0-9a-f]{4,16}\.yaml.*nothing written" <<<"$AI_NEXT" \
    && [[ -z "$(git -C "$PLANT_ROOT" status --porcelain)" ]]; then
    ai_ok "the next ordinal counts past both" "AM-003-<hash>, nothing written by --dry-run"
else
    ai_fail "the next ordinal counts past both" "$AI_NEXT"
fi

# ------------------------------------------------------------------ refuse --
# A name that is not an id, a hash in capitals, a record naming another id,
# and the skeleton left unfilled: each refused by name.
plant_cmd "an amendment named AM-2x" "amendment.id" '"AM-2x" is not an amendment id' 2 \
    "cp '$AI_DIR/AM-001.yaml' '$AI_DIR/AM-2x.yaml' && sed -i 's|^id: .*|id: \"AM-2x\"|' '$AI_DIR/AM-2x.yaml'" \
    check "$AI_ALIAS"
plant_cmd "an amendment hash in capitals" "amendment.id" '"AM-003-C07E" is not an amendment id' 2 \
    "cp '$AI_DIR/AM-001.yaml' '$AI_DIR/AM-003-C07E.yaml' && sed -i 's|^id: .*|id: \"AM-003-C07E\"|' '$AI_DIR/AM-003-C07E.yaml'" \
    check "$AI_ALIAS"
plant_cmd "an amendment whose id is not its name" "amendment.id" "the record's id is \"AM-003\" and its file is AM-003-beef.yaml" 2 \
    "cp '$AI_DIR/AM-001.yaml' '$AI_DIR/AM-003-beef.yaml' && sed -i 's|^id: .*|id: \"AM-003\"|' '$AI_DIR/AM-003-beef.yaml'" \
    check "$AI_ALIAS"
plant_cmd "an unfilled war amend skeleton" "amendment.invalid" "omits reason" 2 \
    "\"$WAR\" --root '$PLANT_ROOT' amend '$AI_ALIAS' >/dev/null 2>&1" \
    check "$AI_ALIAS"
plant_restore
unset PLANT_ROOT

# A resolved Warrant of this corpus: refused by name, nothing written.
AI_RESOLVED=$(ls docs/warrants/*/resolution.toml 2>/dev/null | head -1 | xargs -r dirname | xargs -r basename)
if [[ -n "$AI_RESOLVED" ]]; then
    AI_BEFORE=$(git status --porcelain -- "docs/warrants/$AI_RESOLVED")
    AI_OUT=$("$WAR" amend "$AI_RESOLVED" 2>&1); AI_S=$?
    AI_AFTER=$(git status --porcelain -- "docs/warrants/$AI_RESOLVED")
    if [[ $AI_S -eq 2 && "$AI_BEFORE" == "$AI_AFTER" ]] && grep -q 'amend.resolved' <<<"$AI_OUT"; then
        ai_ok "war amend of a resolved Warrant" "$AI_RESOLVED refused by name, nothing written"
    else
        ai_fail "war amend of a resolved Warrant" "exit $AI_S: $AI_OUT"
    fi
    restore
else
    ai_fail "war amend of a resolved Warrant" "no resolved Warrant in this corpus to try"
fi

# ------------------------------------------------------------------ legacy --
# Every amendment already on record — all named AM-<n> — still validates, none
# is refused for its name, and their order is their number.
AI_FILES=$(ls docs/warrants/*/amendments/*.yaml 2>/dev/null | wc -l)
AI_LIVE=$("$WAR" check 2>&1)
AI_VALID=$(grep -c '^PASS amendment.valid' <<<"$AI_LIVE")
AI_LEGACY_ORDER=$("$WAR" status --json 2>/dev/null | python3 -c "
import json, sys
bad = []
for w in json.load(sys.stdin)['result']['warrants']:
    ids = [a['id'] for a in w.get('amendments', [])]
    if ids != sorted(ids, key=lambda i: int(i.split('-')[1])):
        bad.append(w['alias'] + ':' + ','.join(ids))
print(';'.join(bad) or 'ordered')" 2>/dev/null)
if [[ $AI_FILES -gt 0 && "$AI_VALID" -eq "$AI_FILES" && "$AI_LEGACY_ORDER" == "ordered" ]] \
    && ! grep -q 'amendment.id' <<<"$AI_LIVE"; then
    ai_ok "existing AM-<n> records unchanged" "$AI_VALID of $AI_FILES valid, none refused for its name, each Warrant's in number order"
else
    ai_fail "existing AM-<n> records unchanged" "$AI_VALID valid of $AI_FILES; order: $AI_LEGACY_ORDER; $(grep 'amendment.id' <<<"$AI_LIVE" | head -2 | tr '\n' '|')"
fi
