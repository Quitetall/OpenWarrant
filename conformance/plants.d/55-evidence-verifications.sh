# shellcheck shell=bash
# t-fed6 — a verification does not make evidence stale; the source still does.
#
# Under OW-WAR-0133 a gate that declares no `inputs` is bound to the tree it
# ran over. The independent verifier reads the receipts (in the bundle it is
# handed) and writes its verdicts after them, so every verification used to
# stale every such receipt — and recording again changed the receipts the
# verdicts were made against, a loop with no fixed point. This plant records a
# receipt, then does everything `war verify` writes: a bundle (`--bundle`), a
# verifier's response ingested (`--response`), and a `--run` that keeps its
# response under `verifications/responses/` — uncommitted, then committed —
# and asks that the receipt still counts, naming the tree it ran over. The
# refusals: a source byte changed after the receipt stales it by name, and so
# does a file under `verifications/` that `war verify` does not write.
#
# The verifier is conformance/fixtures/verifier/establishes-all.sh (as in
# 47-bundle-output.sh), run from outside the scratch program, and the response
# `--response` ingests is written outside it too: the only files that land in
# the tree are the ones the tool writes. No signature: no key, no ssh-agent.

echo "== verification keeps evidence (t-fed6) =="
PLANT_ROOT=$(scratch_corpus VR)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
VR_TMP=$(mktemp -d)
VR_A=VR-WAR-0001
command cp conformance/fixtures/verifier/establishes-all.sh "$VR_TMP/verifier.sh"

# ── The agent's work under -euo pipefail: a gate with no declared inputs (the
# tree rule decides) cited by the scaffold Warrant, the fixture verifier
# configured, a source file, and — last — the recorded evidence, all
# committed.
if ! WAR="$REPO_ROOT/${WAR#./}" D="$PLANT_ROOT" A="$VR_A" R="$REPO_ROOT" V="$VR_TMP/verifier.sh" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$VR_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w="docs/warrants/$A"
sed -e 's/^gate_id: .*/gate_id: "plant.tree"/' "$R/docs/gates/ops.echo@1.0.0.yaml" \
    > docs/gates/plant.tree@1.0.0.yaml
grep -q '^gate_id: "plant.tree"' docs/gates/plant.tree@1.0.0.yaml
! grep -q '^inputs:' docs/gates/plant.tree@1.0.0.yaml
grep -q 'gate://software.repo.war-check@1.0.0' "$w/atoms/60-assurance.md"
sed -i 's|gate://software.repo.war-check@1.0.0|gate://plant.tree@1.0.0|' "$w/atoms/60-assurance.md"
grep -q '^verifier_argv = ' openwarrant.toml
sed -i "s|^verifier_argv = .*|verifier_argv = [\"bash\", \"$V\"]|" openwarrant.toml
mkdir -p src
printf 'one\n' > src/lib.txt
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "a tree-bound gate, a verifier, a source file"
"$WAR" --root . evidence record "$A"
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "evidence"
test -z "$(git status --porcelain)"
test ! -e "$w/verifications"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$VR_TMP/setup.log" >&2
    exit 9
fi

vr_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
vr_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
vr_war()  { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
vr_line() { grep -m1 -E 'evidence\.|ERROR' <<<"$1"; }
vr_commit() {
    git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
    git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1
}
VR_W="$PLANT_ROOT/docs/warrants/$VR_A"
VR_V="docs/warrants/$VR_A/verifications"
VR_RECEIPT="$VR_W/gate-runs/plant_tree_1_0_0.receipt.json"
VR_T=$(grep -o '"tree:[0-9a-f]*"' "$VR_RECEIPT" 2>/dev/null | tr -d '"')
[[ -n "$VR_T" ]] || { printf 'PLANT SETUP FAILED: no receipt tree\n' >&2; exit 9; }
VR_ADMIT="evidence.admissible .*plant.tree@1.0.0 .*$VR_T"

# Before any verification, the receipt counts (the control below is not a
# rule that admits nothing).
VR_OUT=$(vr_war check "$VR_A" 2>&1)
if grep -q "$VR_ADMIT" <<<"$VR_OUT" && ! grep -q 'evidence.stale-binding' <<<"$VR_OUT"; then
    vr_ok "the receipt counts" "evidence.admissible naming $VR_T"
else
    vr_fail "the receipt counts" "$(vr_line "$VR_OUT")"
fi

# 1. Accepting case: a bundle, the verifier's answer to it ingested, and a
#    `--run` that keeps its response — uncommitted. Every file `war verify`
#    writes moved, the verdicts were recorded, and the receipt still counts.
vr_war verify "$VR_A" --performer claude --bundle --json > "$VR_TMP/request.json" 2>/dev/null
VR_B=$(ls "$VR_W"/verifications/bundle-*.json 2>/dev/null | head -1)
VR_INGEST=""
if [[ -n "$VR_B" ]]; then
    OPENWARRANT_REVIEWED_PACKETS=$(python3 -c 'import json,sys;print(json.dumps(json.load(open(sys.argv[1]))["result"]["packets"]))' "$VR_TMP/request.json") \
    bash "$VR_TMP/verifier.sh" "$VR_B" > "$VR_TMP/response.toml" 2>"$VR_TMP/verifier.err"
    VR_INGEST=$(vr_war verify "$VR_A" --response "$VR_TMP/response.toml" 2>&1)
fi
VR_RUN=$(vr_war verify "$VR_A" --performer claude --run 2>&1)
VR_MOVED=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)
VR_OUT=$(vr_war check "$VR_A" 2>&1)
if grep -q 'verify.recorded' <<<"$VR_INGEST" \
    && ! grep -q '^ERROR' <<<"$VR_RUN" \
    && grep -qE "$VR_V/OBL-[0-9]+\.toml" <<<"$VR_MOVED" \
    && grep -qE "$VR_V/bundle-[0-9a-f]{16}\.json" <<<"$VR_MOVED" \
    && grep -qE "$VR_V/responses/response-[0-9a-f]{16}\.toml" <<<"$VR_MOVED" \
    && grep -q "docs/warrants/$VR_A/journal.jsonl" <<<"$VR_MOVED" \
    && grep -q "$VR_ADMIT" <<<"$VR_OUT" && ! grep -q 'evidence.stale-binding' <<<"$VR_OUT"; then
    vr_ok "verification leaves the receipt" "--bundle, --response, --run wrote records, a bundle, a response; still admissible"
else
    vr_fail "verification leaves the receipt" "ingest: $(grep -m1 -E 'verify\.|ERROR' <<<"$VR_INGEST"); run: $(grep -m1 '^ERROR' <<<"$VR_RUN"); $(vr_line "$VR_OUT")"
fi

# ... and committed, as an agent does before the sitting: requirement 5 still
# met, and the obligations established by the verdicts just written.
vr_commit "verification"
VR_OUT=$(vr_war check "$VR_A" 2>&1)
VR_RES=$(vr_war resolve --dry-run "$VR_A" 2>&1)
if grep -q "$VR_ADMIT" <<<"$VR_OUT" && ! grep -q 'evidence.stale-binding' <<<"$VR_OUT" \
    && grep -q 'resolution.requirement-met .*every required gate has admissible result' <<<"$VR_RES"; then
    vr_ok "committed verification leaves it" "a commit of verifications/ after the receipt; requirement 5 met"
else
    vr_fail "committed verification leaves it" "$(vr_line "$VR_OUT") $(grep -m1 'every required gate' <<<"$VR_RES")"
fi

# 2. Refusal: a source byte changed after the receipt stales it by name — the
#    verification records beside it neither hide the change nor are named.
printf 'changed after the receipt\n' >> "$PLANT_ROOT/src/lib.txt"
VR_OUT=$(vr_war check "$VR_A" 2>&1)
VR_RES=$(vr_war resolve --dry-run "$VR_A" 2>&1)
git -C "$PLANT_ROOT" checkout -q -- src/lib.txt
if grep -q "evidence.stale-binding .*names $VR_T .*src/lib.txt" <<<"$VR_OUT" \
    && ! grep -q 'evidence.stale-binding .*verifications' <<<"$VR_OUT" \
    && grep -q 'resolution.requirement-unmet .*every required gate has admissible result' <<<"$VR_RES"; then
    vr_ok "a source change still stales it" "evidence.stale-binding names $VR_T and src/lib.txt only; requirement 5 unmet"
else
    vr_fail "a source change still stales it" "$(vr_line "$VR_OUT")"
fi

# ... and a file under verifications/ that `war verify` does not write is not
# a verification record: it stales the receipt by name.
printf 'a note beside the verdicts\n' > "$VR_W/verifications/notes.md"
VR_OUT=$(vr_war check "$VR_A" 2>&1)
command rm -f "$VR_W/verifications/notes.md"
if grep -q "evidence.stale-binding .*names $VR_T .*$VR_V/notes.md" <<<"$VR_OUT"; then
    vr_ok "another verifications/ file stales it" "evidence.stale-binding names $VR_V/notes.md"
else
    vr_fail "another verifications/ file stales it" "$(vr_line "$VR_OUT")"
fi

command rm -rf "$VR_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
