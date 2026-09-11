# shellcheck shell=bash
# Attestations (OW-ADR-0015): an ssh-signed act leaves a DSSE envelope that
# verifies; a moved byte, an edited subject, an unknown principal and a
# replayed response signature are each refused by name.
#
# The whole flow runs against a throwaway ssh-agent with a key generated here,
# an allowed_signers and roles.toml written here (both restored by `restore`),
# on OW-WAR-0063, which awaits authorization. Nothing here is the owner's key.

ATT_TMP=$(mktemp -d)
ssh-keygen -q -t ed25519 -N "" -C plant -f "$ATT_TMP/id_plant"
PLANT_PUB=$(cut -d' ' -f1,2 "$ATT_TMP/id_plant.pub")
cp docs/authority/allowed_signers "$ATT_TMP/allowed_signers.orig" 2>/dev/null || true
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$PLANT_PUB" > docs/authority/allowed_signers
cat >> docs/authority/roles.toml <<'ROLES'

[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/67-attest.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the attestation plants run."
ssh_principal = "plant"
ROLES
assert_present 'Plant Signer' docs/authority/roles.toml

eval "$(ssh-agent -s > "$ATT_TMP/agent.env"; cat "$ATT_TMP/agent.env")" >/dev/null
ssh-add -q "$ATT_TMP/id_plant" 2>/dev/null

ATT_DIR=docs/warrants/OW-WAR-0063/attestations
ATT_FILE=$ATT_DIR/authorize-1.dsse.json
SIGN_OUT=$("$WAR" sign OW-WAR-0063 --ssh-sign --as "Plant Signer" 2>&1)
if [[ -f "$ATT_FILE" ]] && grep -Fq 'attest.emitted' <<< "$SIGN_OUT"; then
    printf 'ok    %-34s %s written after an ssh-signed authorization\n' "an ssh-signed act is attested" "$ATT_FILE"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no attestation after `war sign --ssh-sign`:\n%s\n' "an ssh-signed act is attested" "$(grep -vE "^[│┌└]" <<< "$SIGN_OUT" | tail -8)"
    FAILED=$((FAILED + 1))
fi

attest_expect() {
    local name="$1" rule="$2" want_exit="$3"
    local out
    out=$("$WAR" attest OW-WAR-0063 --verify 2>&1)
    local status=$?
    if [[ $status -eq $want_exit ]] && grep -Fq -- "$rule" <<< "$out"; then
        printf 'ok    %-34s %s (exit %s)\n' "$name" "$rule" "$status"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s wanted %s exit %s; got exit %s:\n%s\n' "$name" "$rule" "$want_exit" "$status" "$(grep -E '^(ERROR|WARN|UNKNOWN|ok|PASS)' <<< "$out" | head -4)"
        FAILED=$((FAILED + 1))
    fi
}

if [[ -f "$ATT_FILE" ]]; then
    cp "$ATT_FILE" "$ATT_TMP/good.json"
    attest_expect "the attestation verifies" "attest.verified" 0

    # The payload changes while staying a well-formed Statement (the act name
    # in the predicate): the signature no longer covers it.
    python3 - "$ATT_FILE" <<'PY'
import json, sys, base64
p = sys.argv[1]; v = json.load(open(p))
st = json.loads(base64.b64decode(v["payload"])); st["predicate"]["act"] = "authorized-by-nobody"
v["payload"] = base64.b64encode(json.dumps(st, separators=(",", ":")).encode()).decode()
json.dump(v, open(p, "w"))
PY
    attest_expect "a moved payload byte is refused" "attest.failed" 2
    cp "$ATT_TMP/good.json" "$ATT_FILE"

    # The record it attests is edited afterwards.
    printf '\n# edited after attestation\n' >> docs/warrants/OW-WAR-0063/authorization.toml
    attest_expect "an edited subject is refused" "attest.subject-drift" 2
    git checkout -- docs/warrants/OW-WAR-0063/authorization.toml 2>/dev/null || sed -i '/edited after attestation/d' docs/warrants/OW-WAR-0063/authorization.toml
    sed -i '/^# edited after attestation$/d' docs/warrants/OW-WAR-0063/authorization.toml

    # The signer's principal leaves the register.
    sed -i 's/^ssh_principal = "plant"$/# ssh_principal removed by the plant/' docs/authority/roles.toml
    assert_gone 'ssh_principal = "plant"' docs/authority/roles.toml
    attest_expect "an unknown principal is refused" "attest.unknown-principal" 2
    sed -i 's/^# ssh_principal removed by the plant$/ssh_principal = "plant"/' docs/authority/roles.toml

    # The response's own signature (namespace oh.war/response) replayed as the DSSE signature.
    RESP_SIG=$(ls docs/authority/responses/OW-WAR-0063*.response.toml.sig | head -1)
    python3 - "$ATT_FILE" "$RESP_SIG" <<'PY'
import json, sys
p, sig = sys.argv[1], sys.argv[2]
v = json.load(open(p))
body = "".join(l.strip() for l in open(sig) if not l.startswith("-----"))
v["signatures"][0]["sig"] = body; json.dump(v, open(p, "w"))
PY
    attest_expect "a replayed response signature is refused" "attest.failed" 2
    cp "$ATT_TMP/good.json" "$ATT_FILE"
fi

# Cleanup: the agent, the untracked files the act wrote, then the tracked ones.
ssh-agent -k >/dev/null 2>&1 || true
unset SSH_AUTH_SOCK SSH_AGENT_PID
rm -rf "$ATT_DIR" docs/authority/responses/OW-WAR-0063* "$ATT_TMP"
rm -f docs/warrants/OW-WAR-0063/authorization.toml docs/warrants/OW-WAR-0063/judgments.toml
git checkout -- docs/warrants/OW-WAR-0063/ 2>/dev/null || true
restore
