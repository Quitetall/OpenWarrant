# shellcheck shell=bash
# t-cee5 — `war prepare`: a Warrant taken to its sign-off unattended.
#
# One scratch Warrant with a delivered-but-unrecorded file, a gate-executed
# service stage (STAGE-002, gate://plant.stage), a tree-bound gate
# (plant.tree, argv `true`: runs beside others) and `war check --generated`
# (argv `war`: runs alone, after `war compile`), and the fixture verifier
# conformance/fixtures/verifier/establishes-all.sh as `[verify]`.
#
# Refused / stopped, in the order the owner meets them:
#   1. before any signature, prepare delivers and records evidence but the
#      stage waits on the authorization (§47): it stops before verification,
#      names the stage, signs nothing, and the queue still shows `authorize`;
#   2. authorized (a throwaway signer — the sitting's first act), with the
#      tree-bound gate made to fail and committed: the Warrant stops at that
#      gate, by name, the later gate and the verifier do not run, exit 0;
# Accepted:
#   3. the gate restored: from delivered to every §56.1 requirement met,
#      unattended — the stage's dispatch-bound receipt, both cited gates
#      admissible, both obligations established by the independent fixture
#      verifier — and still no resolution: the queue shows `resolve`;
#   4. run again, it is all current and writes nothing;
# Refused:
#   5. resolved (the throwaway signer again), the Warrant is skipped by name
#      and `war deliver` refuses it, both leaving deliverables.toml unmoved.
#
# Every `war prepare` runs with no ssh-agent in its environment. The signer's
# agent is started, asserted to hold only the plant key, and killed around
# each signature. Nothing here reaches the owner's agent or this repository.

echo "== war prepare (t-cee5) =="
PLANT_ROOT=$(scratch_corpus PP)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
PP_TMP=$(mktemp -d)
PP_A=PP-WAR-0001
PP_W="$PLANT_ROOT/docs/warrants/$PP_A"
PP_WAR="$REPO_ROOT/${WAR#./}"

if ! WAR="$PP_WAR" D="$PLANT_ROOT" T="$PP_TMP" A="$PP_A" R="$REPO_ROOT" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$PP_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w="docs/warrants/$A"
for id in plant.tree plant.stage; do
    sed -e "s/^gate_id: .*/gate_id: \"$id\"/" "$R/docs/gates/ops.echo@1.0.0.yaml" > "docs/gates/$id@1.0.0.yaml"
    grep -q "^gate_id: \"$id\"" "docs/gates/$id@1.0.0.yaml"
    grep -q '^argv: \["true"\]' "docs/gates/$id@1.0.0.yaml"
done
grep -q '^argv: \["war", "check", "--generated"\]' docs/gates/software.repo.war-check@1.0.0.yaml
sed -i "s|^argv: .*|argv: [\"$WAR\", \"check\", \"--generated\"]|" docs/gates/software.repo.war-check@1.0.0.yaml
python3 - "$w/atoms/60-assurance.md" "$w/atoms/45-milestones.yaml" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.split("## Acceptance Obligations")[0] + """## Acceptance Obligations

### OBL-001 — the scratch file is delivered and the stage ran
- **scope:** one file in this scratch program, and STAGE-002's run.
- **gate:** `gate://plant.tree@1.0.0`
- **evidence:** the file's bytes; STAGE-002's dispatch-bound receipt.

### OBL-002 — the record is well-formed
- **scope:** this scratch program's corpus.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check --generated` passes.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can this pass with the file wrong? Yes; it is a plant.

- **outcome:** no_counterexample

## Residual Risk

- None; a plant.
"""
open(p, "w").write(s)
p = sys.argv[2]; s = open(p).read()
old = '''  - id: "STAGE-002"
    title: "record evidence; request verification; request resolution"
    executor_kind: "agent"
    responsibility_tier: "T2"'''
assert old in s
s = s.replace(old, '''  - id: "STAGE-002"
    title: "run the stage gate"
    executor_kind: "service"
    responsibility_tier: "T1"
    executor_ref: "gate://plant.stage@1.0.0"
    wall_time_seconds: 5''')
open(p, "w").write(s)
PY
mkdir -p src
printf '%s\n' "$A" > "src/$A.txt"
cat > "$w/deliverables.toml" <<DELIV
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "the plant file"
kind = "file"
target_ref = "src/$A.txt"
required = true
content_addressed = false
provenance_required = true
obligation_refs = ["OBL-001"]
DELIV
printf 'schema = "oh.war/rationale/v1"\n' > "$w/rationale.toml"
cp "$R/conformance/fixtures/verifier/establishes-all.sh" "$T/verifier.sh"
grep -q '^verifier_argv = ' openwarrant.toml
sed -i "s|^verifier_argv = .*|verifier_argv = [\"bash\", \"$T/verifier.sh\"]|" openwarrant.toml
sed -i "s|^verifier_timeout_secs = .*|verifier_timeout_secs = 120|" openwarrant.toml
ssh-keygen -q -t ed25519 -N "" -C plant -f "$T/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$T/id_plant.pub")" > docs/authority/allowed_signers
cat > docs/authority/roles.toml <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/55-prepare.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "plant"
ROLES
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "a Warrant with a gate-executed stage, delivered but not recorded"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$PP_TMP/setup.log" >&2
    exit 9
fi

pp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
pp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
pp_git()  { git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
# `war prepare` as an agent runs it: no ssh-agent, no terminal, and a git
# identity for the one commit it makes.
pp_war()  {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
        GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid \
        GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid \
        "$PP_WAR" --root "$PLANT_ROOT" "$@" </dev/null
}
pp_line() { grep -m1 -E '^(PASS|WARN|ERROR|UNKNOWN) +prepare\.' <<<"$1"; }
pp_sha()  { sha256sum "$1" | cut -d' ' -f1; }
# pp_sign  ->  one batch of whatever awaits, from a throwaway agent that
# holds the plant key and nothing else, killed after.
pp_sign() {
    local old=${SSH_AUTH_SOCK:-} held want out
    unset SSH_AUTH_SOCK SSH_AGENT_PID
    eval "$(ssh-agent -s)" >/dev/null
    ssh-add -q "$PP_TMP/id_plant" 2>/dev/null
    want=$(ssh-keygen -lf "$PP_TMP/id_plant.pub" | awk '{print $2}')
    held=$(ssh-add -l 2>/dev/null)
    if [[ $(grep -c . <<<"$held") -ne 1 || "$(awk '{print $2}' <<<"$held")" != "$want" ]]; then
        printf 'PLANT SETUP FAILED: the agent holds another key\n' >&2
        ssh-agent -k >/dev/null 2>&1
        exit 9
    fi
    out=$("$PP_WAR" --root "$PLANT_ROOT" sign --batch --ssh-sign --as "Plant Signer" </dev/null 2>&1)
    ssh-agent -k >/dev/null 2>&1 || true
    if [[ -n "$old" ]]; then export SSH_AUTH_SOCK="$old"; else unset SSH_AUTH_SOCK; fi
    unset SSH_AGENT_PID
    printf '%s' "$out"
}

# 1. Before any signature: delivered, evidence recorded, the stage waiting on
#    the authorization — stopped before the verifier, nothing signed.
PP_OUT=$(pp_war prepare "$PP_A" --jobs 2 2>&1); PP_STATUS=$?
PP_LIST=$(pp_war sign --list 2>&1)
if [[ $PP_STATUS -eq 0 ]] \
    && grep -q "prepare.awaits-authorization .*$PP_A: deliver done (1 recorded, 0 current) .*run STAGE-002 skipped .*evidence plant.tree@1.0.0 done, software.repo.war-check@1.0.0 done .*STAGE-002 wait" <<<"$PP_OUT" \
    && grep -q '^content_addressed = true$' "$PP_W/deliverables.toml" \
    && PP_SUBJECT=$(pp_git log -1 --format=%s) && grep -q '^prepare: delivery provenance' <<<"$PP_SUBJECT" \
    && [[ ! -e "$PP_W/verifications" && ! -e "$PP_W/authorization.toml" ]] \
    && grep -q "$PP_A .*authorize" <<<"$PP_LIST"; then
    pp_ok "unauthorized: stops before verify" "delivered, committed, evidence recorded; STAGE-002 waits (§47); queue: authorize"
else
    pp_fail "unauthorized: stops before verify" "exit $PP_STATUS: $(pp_line "$PP_OUT")"
fi

# The sitting's first act, by the throwaway signer. Nothing is committed:
# the tree rule skips what a signature writes.
pp_sign >/dev/null
[[ -f "$PP_W/authorization.toml" ]] || { printf 'PLANT SETUP FAILED: the authorization was not recorded\n' >&2; exit 9; }

# 2. A failing gate stops the Warrant at that gate, by name: the in-tree gate
#    after it and the verifier do not run, and the exit is 0 — a result.
sed -i 's|^argv: \["true"\]|argv: ["false"]|' "$PLANT_ROOT/docs/gates/plant.tree@1.0.0.yaml"
grep -q '^argv: \["false"\]' "$PLANT_ROOT/docs/gates/plant.tree@1.0.0.yaml" || { printf 'PLANT SETUP FAILED: the gate did not change\n' >&2; exit 9; }
pp_git commit -qam "the tree gate fails" >/dev/null
PP_CHECK_RUNS=$(ls "$PP_W"/gate-runs/software_repo_war-check* 2>/dev/null | wc -l)
PP_OUT=$(pp_war prepare "$PP_A" 2>&1); PP_STATUS=$?
if [[ $PP_STATUS -eq 0 ]] \
    && grep -q "prepare.stopped .*run STAGE-002 done .*evidence plant.tree@1.0.0 FAILED, software.repo.war-check@1.0.0 skipped .*stopped: evidence plant.tree@1.0.0 failed" <<<"$PP_OUT" \
    && [[ ! -e "$PP_W/verifications" && ! -e "$PP_W/resolution.toml" ]] \
    && [[ $(ls "$PP_W"/gate-runs/software_repo_war-check* 2>/dev/null | wc -l) -eq $PP_CHECK_RUNS ]]; then
    pp_ok "a failing gate stops it, named" "plant.tree@1.0.0 FAILED; war-check and the verifier not run; exit 0"
else
    pp_fail "a failing gate stops it, named" "exit $PP_STATUS: $(pp_line "$PP_OUT")"
fi
sed -i 's|^argv: \["false"\]|argv: ["true"]|' "$PLANT_ROOT/docs/gates/plant.tree@1.0.0.yaml"
pp_git commit -qam "the tree gate passes again" >/dev/null

# 3. Accepted: from here to every §56.1 requirement met, unattended — and no
#    resolution: that is the human's, and the queue says so.
PP_OUT=$(pp_war prepare "$PP_A" --jobs 2 2>&1); PP_STATUS=$?
PP_RES=$(pp_war resolve --dry-run "$PP_A" 2>&1)
PP_LIST=$(pp_war sign --list 2>&1)
PP_CHECK=$(pp_war check --generated 2>&1); PP_CHECK_STATUS=$?
if [[ $PP_STATUS -eq 0 ]] \
    && grep -q "prepare.prepared .*run STAGE-002 current .*evidence plant.tree@1.0.0 done, software.repo.war-check@1.0.0 done .*verify done (2/2 established) .*§56.1 all 13 met" <<<"$PP_OUT" \
    && grep -q "resolution.requirements .*all 13 §56.1 requirements are met" <<<"$PP_RES" \
    && grep -q "$PP_A .*resolve" <<<"$PP_LIST" && grep -q "$PP_A .*resolve" <<<"$PP_OUT" \
    && [[ ! -e "$PP_W/resolution.toml" && $PP_CHECK_STATUS -eq 0 ]]; then
    pp_ok "delivered to ready, unattended" "stage receipt, both gates admissible, 2/2 established, all 13 met; queue: resolve"
else
    pp_fail "delivered to ready, unattended" "exit $PP_STATUS: $(pp_line "$PP_OUT") | check exit $PP_CHECK_STATUS"
fi

# 4. Again: everything current, nothing written.
PP_STATE=$(cd "$PLANT_ROOT" && git status --porcelain --untracked-files=all | sort | while IFS= read -r l; do f=${l:3}; printf '%s %s\n' "$f" "$(sha256sum "$f" | cut -d' ' -f1)"; done)
PP_OUT=$(pp_war prepare "$PP_A" 2>&1); PP_STATUS=$?
PP_AFTER=$(cd "$PLANT_ROOT" && git status --porcelain --untracked-files=all | sort | while IFS= read -r l; do f=${l:3}; printf '%s %s\n' "$f" "$(sha256sum "$f" | cut -d' ' -f1)"; done)
if [[ $PP_STATUS -eq 0 && "$PP_STATE" == "$PP_AFTER" ]] \
    && grep -q "prepare.prepared .*deliver current .*run STAGE-002 current .*evidence plant.tree@1.0.0 current, software.repo.war-check@1.0.0 current .*verify current" <<<"$PP_OUT"; then
    pp_ok "a second run writes nothing" "every step current; the tree byte-identical"
else
    pp_fail "a second run writes nothing" "exit $PP_STATUS: $(pp_line "$PP_OUT")"
fi

# 5. Resolved by the throwaway signer: skipped by name, and `war deliver`
#    refuses it; deliverables.toml does not move.
pp_sign >/dev/null
[[ -f "$PP_W/resolution.toml" ]] || { printf 'PLANT SETUP FAILED: the resolution was not recorded\n' >&2; exit 9; }
PP_BEFORE=$(pp_sha "$PP_W/deliverables.toml")
PP_OUT=$(pp_war prepare "$PP_A" 2>&1); PP_STATUS=$?
PP_DEL=$(pp_war deliver "$PP_A" 2>&1); PP_DEL_STATUS=$?
if [[ $PP_STATUS -eq 0 && $PP_DEL_STATUS -eq 2 ]] \
    && grep -q "prepare.skipped .*$PP_A: .*$PP_A is resolved" <<<"$PP_OUT" \
    && grep -q "deliver.resolved .*$PP_A is resolved" <<<"$PP_DEL" \
    && [[ "$(pp_sha "$PP_W/deliverables.toml")" == "$PP_BEFORE" ]]; then
    pp_ok "a resolved Warrant is skipped" "prepare.skipped and deliver.resolved by name; deliverables.toml unmoved"
else
    pp_fail "a resolved Warrant is skipped" "prepare exit $PP_STATUS: $(pp_line "$PP_OUT"); deliver exit $PP_DEL_STATUS: $(grep -m1 deliver. <<<"$PP_DEL")"
fi

command rm -rf "$PP_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
