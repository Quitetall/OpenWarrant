# shellcheck shell=bash
# t-ca38 — ticket work leaves the committed projections current.
#
# The report: "since t-67ed the corpus projections include ready ticket items,
# so `war create`/`done` make `war check --generated` fail until someone
# recompiles". Not so: `war next` renders ready items live from the ticket
# store (t-67ed kept `actions`, what CURRENT.md and the timeline read,
# unchanged), and no projection names a ticket. The one path from ticket work
# to a projection is evidence: the corpus status projects each receipt's
# admissibility, and a tree-bound receipt is admissible only while the tree it
# names has not moved — which ticket files do not move (t-5d82). The drift the
# 2026-09-26 evidence run saw came from receipts and verifications recorded
# after the last compile, not from a ticket.
#
# So this plant holds the whole path over a scratch corpus whose projection
# depends on the tree: a tree-bound gate, a recorded receipt, compiled and
# committed. Then:
#   accepting: create, add, claim, note, done — uncommitted, then committed —
#              and `war check --generated` reports no drift
#   refusing:  a source byte changed after the receipt is drift (the
#              projection does read the tree, so the accept is not vacuous);
#              a hand edit of CORPUS_STATUS.md is drift
#
# No signature is involved: no key, no ssh-agent.

echo "== ticket work leaves the projections current (t-ca38) =="
PLANT_ROOT=$(scratch_corpus TP)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
TP_TMP=$(mktemp -d)
TP_A=TP-WAR-0001

if ! WAR="$REPO_ROOT/${WAR#./}" D="$PLANT_ROOT" A="$TP_A" R="$REPO_ROOT" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$TP_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w="docs/warrants/$A"
sed -e 's/^gate_id: .*/gate_id: "plant.tree"/' "$R/docs/gates/ops.echo@1.0.0.yaml" \
    > docs/gates/plant.tree@1.0.0.yaml
grep -q '^gate_id: "plant.tree"' docs/gates/plant.tree@1.0.0.yaml
! grep -q '^inputs:' docs/gates/plant.tree@1.0.0.yaml
grep -q 'gate://software.repo.war-check@1.0.0' "$w/atoms/60-assurance.md"
sed -i 's|gate://software.repo.war-check@1.0.0|gate://plant.tree@1.0.0|' "$w/atoms/60-assurance.md"
mkdir -p src
printf 'one\n' > src/lib.txt
"$WAR" --root . create "Ship the parser" --item "Write the grammar" --item "Wire the CLI" >/dev/null
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "a tree-bound gate, a ticket, a source file"
"$WAR" --root . evidence record "$A"
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "evidence, compiled"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$TP_TMP/setup.log" >&2
    exit 9
fi

tp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
tp_war()  { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
# tp_drift → the drift rules `war check --generated` names, one per line
tp_drift() { tp_war check --generated 2>&1 | grep -oE '^ERROR +[a-z-]+\.drift( +[^ ]+)?' | sed 's/^ERROR *//' | sort -u; }
tp_commit() {
    git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
    git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1
}
TP=$(find "$PLANT_ROOT/docs/tickets" -mindepth 1 -maxdepth 1 -name 't-*' -printf '%f\n' 2>/dev/null | head -1)
TP_I1=$(grep -o '(i-[0-9a-z]*' "$PLANT_ROOT/docs/tickets/$TP/atoms/15-checklist.md" 2>/dev/null | head -1 | tr -d '(')
[[ -n "$TP" && -n "$TP_I1" ]] || { printf 'PLANT SETUP FAILED: no ticket or no item\n' >&2; exit 9; }
TP_RES=$(tp_war resolve --dry-run "$TP_A" 2>&1)
grep -q 'resolution.requirement-met .*every required gate has admissible result' <<<"$TP_RES" \
    || { printf 'PLANT SETUP FAILED: the receipt is not admissible, so no projection reads the tree\n' >&2; exit 9; }

TP_D=$(tp_drift)
if [[ -z "$TP_D" ]]; then
    tp_ok "the baseline is current" "war check --generated: no drift"
else
    tp_fail "the baseline is current" "$(tr '\n' ' ' <<<"$TP_D")"
fi

# 1. Accepting: every ticket act, uncommitted, then committed.
tp_war create "A second ticket" --item "Something later" >/dev/null 2>&1 \
    && tp_war add "$TP" "One more item" >/dev/null 2>&1 \
    && tp_war claim "$TP/$TP_I1" >/dev/null 2>&1 \
    && tp_war note "$TP" "the CLI waits on the grammar" >/dev/null 2>&1 \
    && tp_war done "$TP/$TP_I1" --note "grammar written" >/dev/null 2>&1
TP_LOOP=$?
TP_MOVED=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all -- docs/tickets | wc -l)
TP_D=$(tp_drift)
if [[ $TP_LOOP -eq 0 && $TP_MOVED -ge 3 && -z "$TP_D" ]]; then
    tp_ok "ticket acts leave no drift" "create/add/claim/note/done moved $TP_MOVED ticket file(s); no drift"
else
    tp_fail "ticket acts leave no drift" "loop exit $TP_LOOP, $TP_MOVED moved: $(tr '\n' ' ' <<<"$TP_D")"
fi
tp_commit "ticket work"
TP_D=$(tp_drift)
if [[ -z "$TP_D" ]]; then
    tp_ok "committed, still no drift" "war check --generated: no drift"
else
    tp_fail "committed, still no drift" "$(tr '\n' ' ' <<<"$TP_D")"
fi

# 2. Refusing: the projection reads the tree — a source byte is drift ...
printf 'changed after the receipt\n' >> "$PLANT_ROOT/src/lib.txt"
TP_D=$(tp_drift)
git -C "$PLANT_ROOT" checkout -q -- src/lib.txt
if grep -q '^corpus-status\.drift' <<<"$TP_D"; then
    tp_ok "a source change is drift" "corpus-status.drift: the receipt went stale"
else
    tp_fail "a source change is drift" "no corpus-status.drift: ${TP_D:-nothing}"
fi

# ... and so is a hand edit of a projection.
TP_STATUS="$PLANT_ROOT/docs/warrants/generated/CORPUS_STATUS.md"
printf '\nedited by hand\n' >> "$TP_STATUS"
TP_D=$(tp_drift)
git -C "$PLANT_ROOT" checkout -q -- docs/warrants/generated/CORPUS_STATUS.md
if grep -q '^corpus-status\.drift' <<<"$TP_D"; then
    tp_ok "a hand-edited projection is drift" "corpus-status.drift"
else
    tp_fail "a hand-edited projection is drift" "no corpus-status.drift: ${TP_D:-nothing}"
fi

command rm -rf "$TP_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
