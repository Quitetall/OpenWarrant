# shellcheck shell=bash
# t-5d82 — working a ticket does not make evidence stale; the source still does.
#
# Under OW-WAR-0133 a gate that declares no `inputs` is bound to the tree it
# ran over. The ticket loop writes files as it is worked — `war claim` a lock
# (and a journal line), `war done` a ticked checklist line, `war note` a dated
# line in the intent, `war create` a whole new ticket — so ticket work done
# after the evidence was recorded used to stale every such receipt. This plant
# records a receipt, works tickets (uncommitted, then committed), and asks
# that the receipt still counts, naming the tree it ran over. The refusals: a
# source byte changed after the receipt stales it by name, and so does a file
# in a ticket's directory the loop does not write.
#
# A claims directory ignores itself (the loop writes a `.gitignore` into it),
# so its locks never reach git. Here it is pointed at an in-repository path
# and that `.gitignore` is removed after a claim (each claim writes it back),
# so the held lock does reach the tree rule as
# an untracked file and the exclusion, not the ignore file, is under test.
#
# No signature is involved: no key, no ssh-agent.

echo "== ticket work keeps evidence (t-5d82) =="
PLANT_ROOT=$(scratch_corpus TB)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
TB_TMP=$(mktemp -d)
TB_A=TB-WAR-0001

# ── The agent's work under -euo pipefail: a gate with no declared inputs (the
# tree rule decides) cited by the scaffold Warrant, one ticket, a source file,
# and — last — the recorded evidence, all committed.
if ! WAR="$REPO_ROOT/${WAR#./}" D="$PLANT_ROOT" A="$TB_A" R="$REPO_ROOT" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$TB_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w="docs/warrants/$A"
sed -e 's/^gate_id: .*/gate_id: "plant.tree"/' "$R/docs/gates/ops.echo@1.0.0.yaml" \
    > docs/gates/plant.tree@1.0.0.yaml
grep -q '^gate_id: "plant.tree"' docs/gates/plant.tree@1.0.0.yaml
! grep -q '^inputs:' docs/gates/plant.tree@1.0.0.yaml
grep -q 'gate://software.repo.war-check@1.0.0' "$w/atoms/60-assurance.md"
sed -i 's|gate://software.repo.war-check@1.0.0|gate://plant.tree@1.0.0|' "$w/atoms/60-assurance.md"
! grep -q '^\[tickets\]' openwarrant.toml
printf '\n[tickets]\nclaims_dir = "var/claims"\n' >> openwarrant.toml
mkdir -p src
printf 'one\n' > src/lib.txt
"$WAR" --root . create "Ship the parser" --item "Write the grammar" --item "Wire the CLI" >/dev/null
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "a tree-bound gate, a ticket, a source file"
"$WAR" --root . evidence record "$A"
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "evidence"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$TB_TMP/setup.log" >&2
    exit 9
fi

tb_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tb_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
tb_war()  { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
tb_line() { grep -m1 -E 'evidence\.|ERROR' <<<"$1"; }
tb_commit() {
    git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
    git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1
}
TB_RECEIPT="$PLANT_ROOT/docs/warrants/$TB_A/gate-runs/plant_tree_1_0_0.receipt.json"
TB_T=$(grep -o '"tree:[0-9a-f]*"' "$TB_RECEIPT" 2>/dev/null | tr -d '"')
TB=$(ls "$PLANT_ROOT/docs/tickets" 2>/dev/null | grep -m1 -E '^t-')
[[ -n "$TB_T" && -n "$TB" ]] || { printf 'PLANT SETUP FAILED: no receipt tree or no ticket\n' >&2; exit 9; }
TB_I1=$(grep -o '(i-[0-9a-z]*' "$PLANT_ROOT/docs/tickets/$TB/atoms/15-checklist.md" | head -1 | tr -d '(')
TB_ADMIT="evidence.admissible .*plant.tree@1.0.0 .*$TB_T"

# Before any ticket work, the receipt counts (the control below is not a
# rule that admits nothing).
TB_OUT=$(tb_war check "$TB_A" 2>&1)
if grep -q "$TB_ADMIT" <<<"$TB_OUT" && ! grep -q 'evidence.stale-binding' <<<"$TB_OUT"; then
    tb_ok "the receipt counts" "evidence.admissible naming $TB_T"
else
    tb_fail "the receipt counts" "$(tb_line "$TB_OUT")"
fi

# 1. Accepting case: claim, done, note, a second ticket — uncommitted. Every
#    file the loop wrote moved, and the receipt still counts.
tb_war claim "$TB/$TB_I1" >/dev/null 2>&1 \
    && tb_war done "$TB/$TB_I1" --note "grammar written" >/dev/null 2>&1 \
    && tb_war claim "$TB" >/dev/null 2>&1 \
    && command rm -f "$PLANT_ROOT/var/claims/.gitignore" \
    && tb_war note "$TB" "the CLI waits on the grammar" >/dev/null 2>&1 \
    && tb_war create "A second ticket" --item "Something later" >/dev/null 2>&1
TB_LOOP=$?
TB_MOVED=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)
TB_OUT=$(tb_war check "$TB_A" 2>&1)
if [[ $TB_LOOP -eq 0 ]] \
    && grep -q "docs/tickets/$TB/journal.jsonl" <<<"$TB_MOVED" \
    && grep -q "docs/tickets/$TB/atoms/15-checklist.md" <<<"$TB_MOVED" \
    && grep -q "docs/tickets/$TB/atoms/10-intent.md" <<<"$TB_MOVED" \
    && grep -q "var/claims/$TB.lock" <<<"$TB_MOVED" \
    && grep -q "$TB_ADMIT" <<<"$TB_OUT" && ! grep -q 'evidence.stale-binding' <<<"$TB_OUT"; then
    tb_ok "ticket work leaves the receipt" "claim/done/note/create moved journal, checklist, intent, a lock; still admissible"
else
    tb_fail "ticket work leaves the receipt" "loop exit $TB_LOOP; $(tb_line "$TB_OUT")"
fi

# ... and committed with the work, as an agent does.
tb_commit "ticket work"
TB_OUT=$(tb_war check "$TB_A" 2>&1)
TB_RES=$(tb_war resolve --dry-run "$TB_A" 2>&1)
if grep -q "$TB_ADMIT" <<<"$TB_OUT" && ! grep -q 'evidence.stale-binding' <<<"$TB_OUT" \
    && grep -q 'resolution.requirement-met .*every required gate has admissible result' <<<"$TB_RES"; then
    tb_ok "committed ticket work leaves it" "a commit of ticket files after the receipt; requirement 5 met"
else
    tb_fail "committed ticket work leaves it" "$(tb_line "$TB_OUT") $(grep -m1 'every required gate' <<<"$TB_RES")"
fi

# 2. Refusal: a source byte changed after the receipt stales it by name — the
#    ticket files beside it neither hide the change nor are named with it.
printf 'changed after the receipt\n' >> "$PLANT_ROOT/src/lib.txt"
TB_OUT=$(tb_war check "$TB_A" 2>&1)
TB_RES=$(tb_war resolve --dry-run "$TB_A" 2>&1)
git -C "$PLANT_ROOT" checkout -q -- src/lib.txt
if grep -q "evidence.stale-binding .*names $TB_T .*src/lib.txt" <<<"$TB_OUT" \
    && ! grep -q 'evidence.stale-binding .*docs/tickets' <<<"$TB_OUT" \
    && grep -q 'resolution.requirement-unmet .*every required gate has admissible result' <<<"$TB_RES"; then
    tb_ok "a source change still stales it" "evidence.stale-binding names $TB_T and src/lib.txt only; requirement 5 unmet"
else
    tb_fail "a source change still stales it" "$(tb_line "$TB_OUT")"
fi

# ... and a file in a ticket's directory the loop does not write is not
# bookkeeping: it stales the receipt by name.
printf 'an attachment\n' > "$PLANT_ROOT/docs/tickets/$TB/design.md"
TB_OUT=$(tb_war check "$TB_A" 2>&1)
command rm -f "$PLANT_ROOT/docs/tickets/$TB/design.md"
if grep -q "evidence.stale-binding .*names $TB_T .*docs/tickets/$TB/design.md" <<<"$TB_OUT"; then
    tb_ok "another ticket-dir file stales it" "evidence.stale-binding names docs/tickets/$TB/design.md"
else
    tb_fail "another ticket-dir file stales it" "$(tb_line "$TB_OUT")"
fi

command rm -rf "$TB_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
