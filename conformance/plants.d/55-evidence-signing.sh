# shellcheck shell=bash
# t-22fd — signing does not make evidence stale; the source still does.
#
# Under OW-WAR-0133 a gate that declares no `inputs` is bound to the tree it
# ran over. The records a human act writes (authorization, responses, the
# batch, attestations, the journal line) used to count as a moved tree, so a
# signature recorded after the evidence staled every such receipt and a
# Warrant needed two sittings. This plant is the one sitting: the agent's work
# is finished and committed, then — with nothing committed in between — a
# throwaway signer authorizes, the receipt still counts, and the same signer
# resolves. The refusals: a source byte changed after the receipt stales it by
# name, and so does a trust root (`docs/authority/roles.toml`), which is
# hand-written, not the record of an act.
#
# A scratch program (SK), a key generated here, and a throwaway ssh-agent
# asserted to hold only that key (as 49-authorization-retained.sh). Nothing
# here reaches the owner's agent or this repository's corpus.

echo "== signing keeps evidence (t-22fd) =="
PLANT_ROOT=$(scratch_corpus SK)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
SK_TMP=$(mktemp -d)
SK_A=SK-WAR-0001
SK_W="$PLANT_ROOT/docs/warrants/$SK_A"

# ── The agent's work, under -euo pipefail and without any ssh-agent: a gate
# with no declared inputs (the tree rule decides), one Warrant citing it, the
# recorded evidence and verification of that evidence, all committed.
if ! WAR="$REPO_ROOT/${WAR#./}" D="$PLANT_ROOT" T="$SK_TMP" A="$SK_A" R="$REPO_ROOT" \
    BIND="$REPO_ROOT/conformance/fixtures/verifier/with-subject.py" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$SK_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w="docs/warrants/$A"
sed -e 's/^gate_id: .*/gate_id: "plant.tree"/' "$R/docs/gates/ops.echo@1.0.0.yaml" \
    > docs/gates/plant.tree@1.0.0.yaml
grep -q '^gate_id: "plant.tree"' docs/gates/plant.tree@1.0.0.yaml
! grep -q '^inputs:' docs/gates/plant.tree@1.0.0.yaml
python3 - "$w/atoms/60-assurance.md" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.split("## Acceptance Obligations")[0] + """## Acceptance Obligations

### OBL-001 — the scratch file says resolved
- **scope:** one file in this scratch program.
- **gate:** `gate://plant.tree@1.0.0`
- **evidence:** the file's bytes.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can this pass with the file wrong? Yes; it is a plant.

- **outcome:** no_counterexample

## Residual Risk

- None; a plant.
"""
open(p, "w").write(s)
PY
sed -i 's/"OBL-002"/"OBL-001"/' "$w/atoms/45-milestones.yaml"
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
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001"]

[deliverable.provenance]
producer = "claude"
producing_attempt = "$A/attempt-1"
contract_digest = "unrecorded"
tool_or_runtime_identity = "plant"
creation_method = "authored"
content_digest = "sha256:$(sha256sum "src/$A.txt" | cut -d' ' -f1)"
media_type = "text/plain"
classification = "internal"
retention = "repository-lifetime"
source_holder = "git"
DELIV
printf 'schema = "oh.war/rationale/v1"\n' > "$w/rationale.toml"
ssh-keygen -q -t ed25519 -N "" -C plant -f "$T/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$T/id_plant.pub")" > docs/authority/allowed_signers
cat > docs/authority/roles.toml <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/55-evidence-signing.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "plant"
ROLES
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "one Warrant citing a tree-bound gate"
"$WAR" --root . evidence record "$A"
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "evidence"
"$WAR" --root . verify "$A" --performer claude --json > "$T/request.json"
cat > "$T/verified.toml" <<VERIFY
schema = "oh.war/verification-response/v1"
warrant = "$A"

[[verifications]]
obligation = "OBL-001"
disposition = "established"
evidence = "src/$A.txt read"
performer = "claude"

[verifications.verifier]
actor = "plant-verifier"
kind = "agent"
model = "plant-model"

[verifications.verifier.independence]
performer_transcript_blind = true
performer_rationale_blind = true
separate_writable_workspace = true
cannot_modify_subject_artifacts = true
cannot_modify_gate_definition = true
cannot_modify_gate_fixtures = true
separate_context_compilation = true
distinct_model_required = true
distinct_human_required = false
VERIFY
python3 "$BIND" "$T/request.json" "$T/verified.toml" > "$T/verified-bound.toml"
"$WAR" --root . verify "$A" --response "$T/verified-bound.toml"
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "verified"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$SK_TMP/setup.log" >&2
    exit 9
fi

sk_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sk_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sk_war()  { "$WAR" --root "$PLANT_ROOT" "$@"; }
sk_line() { grep -m1 -E 'evidence\.|ERROR' <<<"$1"; }
SK_RECEIPT="$SK_W/gate-runs/plant_tree_1_0_0.receipt.json"
SK_T=$(grep -o '"tree:[0-9a-f]*"' "$SK_RECEIPT" | tr -d '"')
# Before the sitting only the human requirements are unmet; after the
# authorization all thirteen are, which is what lets the sitting go on.
SK_MET='resolution.requirements .*all 13 §56.1 requirements are met'
SK_UNMET='resolution.requirement-unmet .*every required gate has admissible result'
[[ -n "$SK_T" ]] || { printf 'PLANT SETUP FAILED: the receipt names no tree\n' >&2; exit 9; }

# ── The sitting. A throwaway agent holding the plant key and nothing else.
SK_OLD_SOCK=${SSH_AUTH_SOCK:-}
unset SSH_AUTH_SOCK SSH_AGENT_PID
eval "$(ssh-agent -s)" >/dev/null
ssh-add -q "$SK_TMP/id_plant" 2>/dev/null
SK_WANT=$(ssh-keygen -lf "$SK_TMP/id_plant.pub" | awk '{print $2}')
SK_HELD=$(ssh-add -l 2>/dev/null)
if [[ $(grep -c . <<<"$SK_HELD") -ne 1 || "$(awk '{print $2}' <<<"$SK_HELD")" != "$SK_WANT" ]]; then
    printf 'PLANT SETUP FAILED: the agent holds another key\n' >&2
    ssh-agent -k >/dev/null 2>&1
    exit 9
fi
sk_batch() { sk_war sign --batch --ssh-sign --as "Plant Signer" </dev/null 2>&1; }

# 1. Authorize, in a batch. Nothing is committed from here on: a sitting's
#    records are whatever it wrote.
SK_OUT=$(sk_batch); SK_STATUS=$?
if [[ $SK_STATUS -eq 0 && -f "$SK_W/authorization.toml" ]] \
    && ls "$PLANT_ROOT"/docs/authority/batches/*.json >/dev/null 2>&1 \
    && [[ -n "$(git -C "$PLANT_ROOT" status --porcelain -- docs/authority "docs/warrants/$SK_A")" ]]; then
    sk_ok "the sitting authorizes" "one batch; authorization.toml, the batch and its response written, uncommitted"
else
    sk_fail "the sitting authorizes" "exit $SK_STATUS: $(grep -m2 -E '^ERROR|batch\.' <<<"$SK_OUT" | tr '\n' '|')"
fi

# 2. Accepting case: the receipt recorded before the signature still counts,
#    under the tree rule, naming the tree it ran over.
SK_OUT=$(sk_war check "$SK_A" 2>&1)
SK_RES=$(sk_war resolve --dry-run "$SK_A" 2>&1)
if grep -q "evidence.admissible .*plant.tree@1.0.0 .*$SK_T" <<<"$SK_OUT" \
    && ! grep -q 'evidence.stale-binding' <<<"$SK_OUT" && grep -q "$SK_MET" <<<"$SK_RES"; then
    sk_ok "a signature leaves the receipt" "evidence.admissible naming $SK_T after the authorization; all 13 met"
else
    sk_fail "a signature leaves the receipt" "$(sk_line "$SK_OUT") $(grep -m1 -E 'resolution\.requirement' <<<"$SK_RES")"
fi

# 3. Refusal: a source byte changed after the receipt stales it, by name —
#    the authority records beside it do not hide the change.
printf 'changed after the receipt\n' >> "$PLANT_ROOT/src/$SK_A.txt"
SK_OUT=$(sk_war check "$SK_A" 2>&1)
SK_RES=$(sk_war resolve --dry-run "$SK_A" 2>&1)
git -C "$PLANT_ROOT" checkout -q -- "src/$SK_A.txt"
if grep -q "evidence.stale-binding .*names $SK_T .*src/$SK_A.txt" <<<"$SK_OUT" \
    && ! grep -q "evidence.stale-binding .*authorization" <<<"$SK_OUT" \
    && grep -q "$SK_UNMET" <<<"$SK_RES"; then
    sk_ok "a source change still stales it" "evidence.stale-binding names $SK_T and src/$SK_A.txt only; requirement 5 unmet"
else
    sk_fail "a source change still stales it" "$(sk_line "$SK_OUT")"
fi

# ... and a trust root is not a record of an act: an edit to roles.toml stales
# it too.
printf '# edited by hand after the receipt\n' >> "$PLANT_ROOT/docs/authority/roles.toml"
SK_OUT=$(sk_war check "$SK_A" 2>&1)
git -C "$PLANT_ROOT" checkout -q -- docs/authority/roles.toml
if grep -q "evidence.stale-binding .*names $SK_T .*docs/authority/roles.toml" <<<"$SK_OUT"; then
    sk_ok "a trust-root edit stales it" "evidence.stale-binding names docs/authority/roles.toml"
else
    sk_fail "a trust-root edit stales it" "$(sk_line "$SK_OUT")"
fi

# 4. The same sitting resolves. Resolution becomes pending only once the
#    authorization is recorded, and a batch is one role — so it is the
#    second batch of the sitting, with no agent step between.
SK_LIST=$(sk_war sign --list 2>&1)
SK_OUT=$(sk_batch); SK_STATUS=$?
if grep -q "$SK_A.*resolve" <<<"$SK_LIST" && [[ $SK_STATUS -eq 0 && -f "$SK_W/resolution.toml" ]] \
    && grep -q 'common_outcome = "satisfied"' "$SK_W/resolution.toml"; then
    sk_ok "the same sitting resolves" "second batch: resolution.toml, satisfied, on the receipt recorded before the sitting"
else
    sk_fail "the same sitting resolves" "exit $SK_STATUS: $(grep -m2 -E '^ERROR|batch\.|resolution\.requirement-unmet' <<<"$SK_OUT" | tr '\n' '|')"
fi

ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$SK_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$SK_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
unset SSH_AGENT_PID
command rm -rf "$SK_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
