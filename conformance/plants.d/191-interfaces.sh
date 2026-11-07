# shellcheck shell=bash
# The `interfaces` pack: an interface document in any repository, from what
# this build ships. Each claim beside the input it refuses:
#   a clean document reads; a clause selects by id;
#   a status outside the set, a clause with no verify, and a document with no
#   owner are each refused by name.

echo "== interfaces: the interface document type =="
PLANT_ROOT=$(scratch_corpus IF)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
IF_TMP=$(mktemp -d)
if_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
if_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ifw() { "$WAR" --root "$PLANT_ROOT" "$@"; }

IF_ADD=$(ifw plan types add interfaces 2>&1); IF_ADD_RC=$?
IF_DOC="$PLANT_ROOT/docs/records/interfaces/physics.md"
mkdir -p "$(dirname "$IF_DOC")"
cat > "$IF_TMP/good.md" <<'DOC'
---
format: openwarrant/1.0
profile: interface
version: 1.0.0
owner: hpr
parties: hpr, wmd
summary: "What the engine relies on"
---
# Physics interface

## CL-step · clause — A step advances the world by exactly dt
status: agreed
verify: test
check: cargo test --test contract_step

`World::step(dt)` advances simulated time by `dt`.

## CL-handle · clause — Handles are stable across a step
status: tbd
verify: test
owner: hpr
due: 2026-11-20

Whether a removed handle may be reused is not decided.
DOC
if_try() { sed "$1" "$IF_TMP/good.md" > "$IF_DOC"; ifw check 2>&1; }
IF_GOOD=$(if_try '' ); IF_GOOD_RC=$?
IF_CTX=$(ifw context CL-handle 2>&1); IF_CTX_RC=$?
if [[ $IF_ADD_RC -eq 0 && -f "$PLANT_ROOT/profiles/interface.toml" && $IF_GOOD_RC -eq 0 && $IF_CTX_RC -eq 0 ]] \
    && grep -q 'not decided' <<<"$IF_CTX" && ! grep -q 'advances simulated time' <<<"$IF_CTX"; then
    if_ok "an interface reads, clause by clause" "the pack this build ships; CL-handle alone"
else
    if_fail "an interface reads, clause by clause" "add $IF_ADD_RC, check $IF_GOOD_RC, context $IF_CTX_RC: $(head -2 <<<"$IF_GOOD$IF_CTX" | tr '\n' '|')"
fi

IF_BAD=$(if_try '0,/^status: agreed/s//status: maybe/'); IF_BAD_RC=$?
if [[ $IF_BAD_RC -ne 0 ]] && grep -q 'format.field-shape' <<<"$IF_BAD"; then
    if_ok "a status outside the set" "format.field-shape"
else
    if_fail "a status outside the set" "exit $IF_BAD_RC"
fi
IF_BAD=$(if_try '0,/^verify: test/d'); IF_BAD_RC=$?
if [[ $IF_BAD_RC -ne 0 ]] && grep -q 'format.field-missing' <<<"$IF_BAD"; then
    if_ok "a clause must say how it's proved" "format.field-missing"
else
    if_fail "a clause must say how it's proved" "exit $IF_BAD_RC"
fi
IF_BAD=$(if_try '/^owner: hpr$/d'); IF_BAD_RC=$?
if [[ $IF_BAD_RC -ne 0 ]] && grep -q 'format.field-missing' <<<"$IF_BAD"; then
    if_ok "an interface must have an owner" "format.field-missing"
else
    if_fail "an interface must have an owner" "exit $IF_BAD_RC"
fi

/usr/bin/rm -rf "$IF_TMP"
unset IF_TMP IF_ADD IF_ADD_RC IF_DOC IF_GOOD IF_GOOD_RC IF_CTX IF_CTX_RC IF_BAD IF_BAD_RC
