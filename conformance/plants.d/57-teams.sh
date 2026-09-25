# shellcheck shell=bash
# OW-WAR-0137 — teams: review assignment, and who may sign when more than one
# person could.
#
# A scratch program with three humans and one agent in roles.toml, two
# throwaway keys, and one throwaway ssh-agent per signature, each inside
# `bash -euo pipefail` and each asserting `ssh-add -l` lists that key and
# nothing else before anything is signed. Nothing here is the owner's key.
#
# Every claim is paired with a refusal, and every refusal is checked by the
# RULE that fired: a plant refused by the wrong control proves nothing about
# the control it was meant to exercise.

echo "== teams: review assignment (OW-WAR-0137) =="
PLANT_ROOT=$(scratch_corpus TT)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
TT_TMP=$(mktemp -d)
TT_W="$PLANT_ROOT/docs/warrants/TT-WAR-0001"
TT_U="$PLANT_ROOT/docs/warrants/TT-WAR-0002"
for tt_k in ada ben; do ssh-keygen -q -t ed25519 -N "" -C "plant-$tt_k" -f "$TT_TMP/$tt_k"; done
{
    printf 'ada namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$TT_TMP/ada.pub")"
    printf 'ben namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$TT_TMP/ben.pub")"
} > "$PLANT_ROOT/docs/authority/allowed_signers"

# $1: Ben's roles, $2: Ada's (default: all three). The register is rewritten whole each time, so a revocation
# is a visible edit to the file a human owns, and restoring it is the same.
tt_roles() {
    local all='["authorizer", "resolver", "verifier"]'
    local ben=${1:-$all} ada=${2:-$all}
    cat > "$PLANT_ROOT/docs/authority/roles.toml" <<ROLES
[[assignment]]
actor = "Ada"
actor_kind = "human"
roles = $ada
assigned_by = "conformance/plants.d/57-teams.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "ada"

[[assignment]]
actor = "Ben"
actor_kind = "human"
roles = $ben
assigned_by = "conformance/plants.d/57-teams.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "ben"

[[assignment]]
actor = "claude"
actor_kind = "agent"
roles = ["performer", "resolver", "verifier"]
assigned_by = "conformance/plants.d/57-teams.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Over-granted: the agent refusals below must be by kind, not by a missing role."
ROLES
}
tt_roles
tt_assign() { # verify-list, resolve-list (TOML arrays; empty string omits the key)
    {
        printf 'schema = "oh.war/assignment/v1"\n'
        [[ -n "$1" ]] && printf 'verify = %s\n' "$1"
        [[ -n "$2" ]] && printf 'resolve = %s\n' "$2"
    } > "$TT_W/assignment.toml"
}
"$WAR" --root "$PLANT_ROOT" new "An unassigned Warrant beside the assigned one" >/dev/null 2>&1
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register and a second Warrant" >/dev/null 2>&1

tt_expect() { # name, output, exit, want-exit, pattern
    if [[ "$3" == "$4" ]] && grep -q -- "$5" <<<"$2"; then
        printf 'ok    %-44s %s (exit %s)\n' "$1" "$5" "$3"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-44s wanted %s exit %s; got exit %s: %s\n' "$1" "$5" "$4" "$3" "$(grep -E '(ERROR|WARN|FAIL|error)' <<<"$2" | head -3 | tr '\n' '|')"; FAILED=$((FAILED + 1))
    fi
}
tt_true() { # name, detail, condition-exit
    if [[ "$3" -eq 0 ]]; then
        printf 'ok    %-44s %s\n' "$1" "$2"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-44s %s\n' "$1" "$2"; FAILED=$((FAILED + 1))
    fi
}
# One signature: a fresh agent holding exactly one throwaway key, killed on exit.
tt_ssh() { # key, war-args...
    bash -euo pipefail -c '
        key=$1; shift
        eval "$(ssh-agent -s)" >/dev/null
        trap "ssh-agent -k >/dev/null 2>&1" EXIT
        ssh-add -q "$key" 2>/dev/null
        listed=$(ssh-add -l)
        [[ $(wc -l <<<"$listed") -eq 1 ]] || { echo "PLANT: agent holds more than one key" >&2; exit 97; }
        grep -q "$(ssh-keygen -lf "$key.pub" | cut -d" " -f2)" <<<"$listed" || { echo "PLANT: agent holds the wrong key" >&2; exit 97; }
        "$@"
    ' tt-ssh "$@" 2>&1
}
tt_contract() { "$WAR" --root "$PLANT_ROOT" --json authorize "$1" 2>/dev/null | grep -o '"contract_digest": *"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/'; }
tt_response() { # file, alias, assignment_digest line (or empty)
    cat > "$1" <<RESP
schema = "oh.war/authorization-response/v1"
warrant = "$2"
contract_digest = "$(tt_contract "$2")"
authorizer = "Ada"
acting_role = "authorizer"
meaning = "A planted response."
effective_time = "2026-09-24T00:00:00Z"
independence = "separate_role"
$3
RESP
}

# --- OBL-001: the assignment is what the authorizer signed -------------------

tt_assign '["Ada"]' '["Ben"]'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" --json authorize TT-WAR-0001 2>&1)
if grep -q '"assignment_digest": *"sha256:' <<<"$TT_OUT" && grep -q '"resolve"' <<<"$TT_OUT" && grep -q '"Ben"' <<<"$TT_OUT"; then
    tt_true "the request lists the assignment and digest" "assignment + assignment_digest" 0
else
    tt_true "the request lists the assignment and digest" "request: $(head -c 300 <<<"$TT_OUT" | tr '\n' ' ')" 1
fi

tt_response "$TT_TMP/stale.toml" TT-WAR-0001 'assignment_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" authorize TT-WAR-0001 --response "$TT_TMP/stale.toml" 2>&1); tt_expect "a response signing another assignment" "$TT_OUT" $? 2 'authorize.stale-assignment'
tt_response "$TT_TMP/unsigned.toml" TT-WAR-0001 ''
TT_OUT=$("$WAR" --root "$PLANT_ROOT" authorize TT-WAR-0001 --response "$TT_TMP/unsigned.toml" 2>&1); tt_expect "a response echoing no assignment" "$TT_OUT" $? 2 'authorize.assignment-unsigned'
[[ ! -e "$TT_W/authorization.toml" ]]; tt_true "a refused authorization writes nothing" "no authorization.toml" $?

tt_assign '["Dee"]' '["Ben"]'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "an assigned actor without the role" "$TT_OUT" $? 2 'assignment.role-missing'
tt_assign '["Ada"]' '["claude"]'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "the agent named as resolver" "$TT_OUT" $? 2 'assignment.agent-prohibited'
tt_assign '["claude"]' '["Ben"]'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "the performer as its own verifier" "$TT_OUT" $? 2 'assignment.self-verify'

# The positive: Ada signs, and the journal records the digest she signed.
tt_assign '["Ada"]' '["Ben"]'
TT_DIGEST=$("$WAR" --root "$PLANT_ROOT" --json authorize TT-WAR-0001 2>/dev/null | grep -o '"assignment_digest": *"[^"]*"' | sed 's/.*"\(sha256:[^"]*\)"$/\1/')
TT_OUT=$(tt_ssh "$TT_TMP/ada" "$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --ssh-sign --as Ada </dev/null); TT_STATUS=$?
if [[ $TT_STATUS -eq 0 ]] && grep -q 'authorize.recorded' <<<"$TT_OUT" && [[ -n "$TT_DIGEST" ]] \
    && grep -q "\\\\\"assignment_digest\\\\\":\\\\\"$TT_DIGEST" "$TT_W/journal.jsonl" \
    && grep -q "assignment_digest = \"$TT_DIGEST\"" "$PLANT_ROOT"/docs/authority/responses/TT-WAR-0001*.response.toml; then
    tt_true "the signature covers the assignment" "response and journal carry $TT_DIGEST" 0
else
    tt_true "the signature covers the assignment" "exit $TT_STATUS: $(grep -E '(ERROR|PLANT)' <<<"$TT_OUT" | head -2 | tr '\n' '|')" 1
fi
# The unassigned Warrant is signed too, as before this Warrant.
TT_OUT=$(tt_ssh "$TT_TMP/ada" "$WAR" --root "$PLANT_ROOT" sign TT-WAR-0002 --ssh-sign --as Ada </dev/null); tt_expect "an unassigned Warrant authorizes as before" "$TT_OUT" $? 0 'authorize.recorded'

# Each Warrant gets a resolution recorded without a signature, so a resolve
# act awaits one — the act an assignment narrows.
for tt_a in TT-WAR-0001 TT-WAR-0002; do
    tt_d="$PLANT_ROOT/docs/warrants/$tt_a"
    tt_c=$(sed -n 's/^contract_digest = "\(.*\)"/\1/p' "$tt_d/authorization.toml" | head -1)
    cat > "$tt_d/resolution.toml" <<RES
schema = "oh.war/resolution/v1"
warrant = "$tt_a"

[resolution]
id = "01a0d000-0000-7000-8000-00000000000${tt_a: -1}"
common_outcome = "satisfied"
profile_outcome = "delivered"
contract_revision = 1
contract_digest = "$tt_c"
assurance_case_snapshot_digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
artifact_manifest_digest = "absent"
gate_run_refs = []
judgment_refs = []
residual_risk_refs = []
resolved_by_ref = "person://Ben"
acting_role_ref = "role-assignment://Ben/resolver"
meaning = "Planted: recorded without a signature so that a resolve act awaits one."
effective_at = "2026-09-24T00:00:00Z"
recorded_at = "2026-09-24T00:00:00Z"
standing = "valid"
RES
done
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "signed; resolutions await signatures" >/dev/null 2>&1

# --- OBL-002: assignment narrows who may sign, and never widens it -----------

TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "an eligible resolver who is not assigned" "$TT_OUT" $? 2 'sign.not-assigned'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --dry-run 2>&1)
if grep -q 'sign.signer' <<<"$TT_OUT" && grep -q 'signs as Ben, as assigned' <<<"$TT_OUT"; then
    tt_true "no --as selects the assigned resolver" "sign.signer: Ben" 0
else
    tt_true "no --as selects the assigned resolver" "$(grep -E '(ERROR|sign\.)' <<<"$TT_OUT" | head -3 | tr '\n' '|')" 1
fi
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0002 --dry-run 2>&1); tt_expect "unassigned: both eligible, as before" "$TT_OUT" $? 2 'more than one eligible signer (Ada, Ben)'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign --list 2>&1)
if grep -q 'TT-WAR-0001  resolve .*\[assigned: Ben\]' <<<"$TT_OUT" && grep 'TT-WAR-0002  resolve' <<<"$TT_OUT" | grep -qv '\[assigned'; then
    tt_true "the list shows who an act is assigned to" "[assigned: Ben] on 0001 only" 0
else
    tt_true "the list shows who an act is assigned to" "$(grep 'resolve' <<<"$TT_OUT" | tr '\n' '|')" 1
fi

# Ben loses the resolver role. Nobody may sign; Ada is not substituted.
tt_roles '["authorizer", "verifier"]'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --dry-run 2>&1); tt_expect "a revoked assignee makes nobody eligible" "$TT_OUT" $? 2 'assignment.role-revoked'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "and nobody else is added" "$TT_OUT" $? 2 'sign.not-assigned'
tt_roles

# An edit after signing, and a removal (which would widen), refuse the act.
cp "$TT_W/assignment.toml" "$TT_TMP/assignment.keep"
tt_assign '["Ada"]' '["Ada"]'
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "an assignment edited after signing" "$TT_OUT" $? 2 'assignment.moved'
rm -f "$TT_W/assignment.toml"
TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "an assignment removed after signing" "$TT_OUT" $? 2 'assignment.moved'
# The journal is a file an agent can edit too. Strip the recorded digest from
# it as well: the signed response still echoes the assignment, so the removal
# is still refused rather than read as "never assigned".
cp "$TT_W/journal.jsonl" "$TT_TMP/journal.keep"
sed -i 's/,\\"assignment_digest\\":\\"sha256:[0-9a-f]*\\"//' "$TT_W/journal.jsonl"
if grep -q 'assignment_digest' "$TT_W/journal.jsonl"; then
    tt_true "removal with the journal field stripped too" "PLANT: the sed stripped nothing" 1
else
    TT_OUT=$("$WAR" --root "$PLANT_ROOT" sign TT-WAR-0001 --as Ada --dry-run 2>&1); tt_expect "removal with the journal field stripped too" "$TT_OUT" $? 2 'assignment.moved'
fi
cp "$TT_TMP/journal.keep" "$TT_W/journal.jsonl"
cp "$TT_TMP/assignment.keep" "$TT_W/assignment.toml"

# --- OBL-003: only the assigned verifier's verdict is recorded ---------------

tt_verdict() { # file, alias, obligation, actor, kind
    cat > "$1" <<VER
schema = "oh.war/verification-response/v1"
warrant = "$2"

[[verifications]]
obligation = "$3"
disposition = "established"
evidence = "planted: the scratch Warrant's records, read by $4"
performer = "claude"

[verifications.verifier]
actor = "$4"
kind = "${5:-human}"

[verifications.verifier.independence]
performer_transcript_blind = true
performer_rationale_blind = true
separate_writable_workspace = true
cannot_modify_subject_artifacts = true
cannot_modify_gate_definition = true
cannot_modify_gate_fixtures = true
separate_context_compilation = true
distinct_model_required = false
distinct_human_required = true
VER
}
tt_verdict "$TT_TMP/v-ben.toml" TT-WAR-0001 OBL-002 Ben
TT_OUT=$("$WAR" --root "$PLANT_ROOT" verify TT-WAR-0001 --response "$TT_TMP/v-ben.toml" 2>&1); tt_expect "a verdict from an unassigned verifier" "$TT_OUT" $? 2 'verify.not-assigned'
[[ ! -e "$TT_W/verifications/OBL-002.toml" ]]; tt_true "the refused verdict is not written" "no verifications/OBL-002.toml" $?
tt_roles "" '["authorizer", "resolver"]'
tt_verdict "$TT_TMP/v-ada2.toml" TT-WAR-0001 OBL-002 Ada
TT_OUT=$("$WAR" --root "$PLANT_ROOT" verify TT-WAR-0001 --response "$TT_TMP/v-ada2.toml" 2>&1); tt_expect "the assigned verifier without the role" "$TT_OUT" $? 2 'verify.role-missing'
[[ ! -e "$TT_W/verifications/OBL-002.toml" ]]; tt_true "the role-less verdict is not written" "no verifications/OBL-002.toml" $?
tt_roles
tt_assign '["Ben"]' '["Ben"]'
tt_verdict "$TT_TMP/v-ben1.toml" TT-WAR-0001 OBL-001 Ben
TT_OUT=$("$WAR" --root "$PLANT_ROOT" verify TT-WAR-0001 --response "$TT_TMP/v-ben1.toml" 2>&1); tt_expect "a verdict under an assignment edited after" "$TT_OUT" $? 2 'assignment.moved'
[[ ! -e "$TT_W/verifications/OBL-001.toml" ]]; tt_true "the moved assignment writes nothing" "no verifications/OBL-001.toml" $?
cp "$TT_TMP/assignment.keep" "$TT_W/assignment.toml"
tt_verdict "$TT_TMP/v-ada.toml" TT-WAR-0001 OBL-001 Ada
TT_OUT=$("$WAR" --root "$PLANT_ROOT" verify TT-WAR-0001 --response "$TT_TMP/v-ada.toml" 2>&1); tt_expect "the assigned verifier's verdict" "$TT_OUT" $? 0 'verify.recorded'
[[ -f "$TT_W/verifications/OBL-001.toml" ]]; tt_true "the assigned verdict is written" "verifications/OBL-001.toml" $?
tt_verdict "$TT_TMP/v-ben-u.toml" TT-WAR-0002 OBL-001 Ben
TT_OUT=$("$WAR" --root "$PLANT_ROOT" verify TT-WAR-0002 --response "$TT_TMP/v-ben-u.toml" 2>&1); tt_expect "unassigned: any distinct verifier, as before" "$TT_OUT" $? 0 'verify.recorded'

command rm -rf "$TT_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
