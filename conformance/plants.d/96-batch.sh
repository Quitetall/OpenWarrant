# shellcheck shell=bash
# OW-WAR-0072 — the batch act: one signature over many pending acts.
#
# A scratch program with two unsigned Warrants, a key generated here, and a
# throwaway ssh-agent holding it. Nothing here is the owner's key, and nothing
# depends on which of this repository's Warrants are unsigned.

echo "== batch act (OW-WAR-0072) =="
PLANT_ROOT=$(scratch_corpus BT)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
BT_TMP=$(mktemp -d)
ssh-keygen -q -t ed25519 -N "" -C plant -f "$BT_TMP/id_plant"
BT_PUB=$(cut -d' ' -f1,2 "$BT_TMP/id_plant.pub")
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$BT_PUB" > "$PLANT_ROOT/docs/authority/allowed_signers"
cat > "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/96-batch.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the batch plants run."
ssh_principal = "plant"
ROLES
"$WAR" --root "$PLANT_ROOT" new "A second Warrant for the batch" >/dev/null 2>&1
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register and a second Warrant" >/dev/null 2>&1
BT_OLD_SOCK=${SSH_AUTH_SOCK:-}
eval "$(ssh-agent -s > "$BT_TMP/agent.env"; cat "$BT_TMP/agent.env")" >/dev/null
ssh-add -q "$BT_TMP/id_plant" 2>/dev/null
BT_PENDING=$("$WAR" --root "$PLANT_ROOT" sign --list 2>&1 | grep -c 'authorize')

bt_expect() { # name, output, exit, want-exit, pattern
    if [[ "$3" == "$4" ]] && grep -q -- "$5" <<<"$2"; then
        printf 'ok    %-34s %s (exit %s)\n' "$1" "$5" "$3"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s wanted %s exit %s; got exit %s: %s\n' "$1" "$5" "$4" "$3" "$(grep -E '^(ERROR|WARN)' <<<"$2" | head -2 | tr '\n' '|')"; FAILED=$((FAILED + 1))
    fi
}

# Refusals, each by name and each before anything is written.
BT_OUT=$("$WAR" --root "$PLANT_ROOT" sign --batch --as "Plant Signer" 2>&1); bt_expect "a batch without --ssh-sign" "$BT_OUT" $? 2 'batch.needs-ssh'
BT_OUT=$("$WAR" --root "$PLANT_ROOT" sign --batch BT-WAR-0099 --ssh-sign --as "Plant Signer" 2>&1); bt_expect "a target that awaits nothing" "$BT_OUT" $? 2 'batch.unknown-act'
BT_OUT=$("$WAR" --root "$PLANT_ROOT" sign --batch BT-WAR-0001,BT-WAR-0001 --ssh-sign --as "Plant Signer" 2>&1); bt_expect "an act named twice" "$BT_OUT" $? 2 'batch.duplicate-act'
if [[ -z "$(ls "$PLANT_ROOT/docs/authority/batches" 2>/dev/null)" ]] && [[ -z "$(ls "$PLANT_ROOT"/docs/authority/responses/*.toml 2>/dev/null)" ]]; then
    printf 'ok    %-34s no batch, no response left behind\n' "a refused batch writes nothing"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s files left: %s\n' "a refused batch writes nothing" "$(ls "$PLANT_ROOT"/docs/authority/batches "$PLANT_ROOT"/docs/authority/responses 2>/dev/null | tr '\n' ' ')"; FAILED=$((FAILED + 1))
fi

# The dry run judges the whole batch and writes nothing; the `--batch=<list>`
# spelling is the one `war ui` sends, so it is judged the same way.
BT_OUT=$("$WAR" --root "$PLANT_ROOT" sign --batch=BT-WAR-0001,BT-WAR-0002 --as "Plant Signer" --dry-run 2>&1); bt_expect "a batch dry run" "$BT_OUT" $? 0 'batch.would-record'
if [[ -z "$(ls "$PLANT_ROOT/docs/authority/batches" 2>/dev/null)" ]] && [[ -z "$(ls "$PLANT_ROOT"/docs/authority/responses/*.toml 2>/dev/null)" ]]; then
    printf 'ok    %-34s no batch, no response left behind\n' "a batch dry run writes nothing"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s files left: %s\n' "a batch dry run writes nothing" "$(ls "$PLANT_ROOT"/docs/authority/batches "$PLANT_ROOT"/docs/authority/responses 2>/dev/null | tr '\n' ' ')"; FAILED=$((FAILED + 1))
fi

# The positive: every pending authorization, one signature, all recorded.
BT_OUT=$("$WAR" --root "$PLANT_ROOT" sign --batch --ssh-sign --as "Plant Signer" </dev/null 2>&1)
BT_STATUS=$?
BT_DOC=$(ls "$PLANT_ROOT"/docs/authority/batches/*.json 2>/dev/null | grep -v refused | head -1)
BT_AUTH=$(ls "$PLANT_ROOT"/docs/warrants/*/authorization.toml 2>/dev/null | wc -l)
if [[ $BT_STATUS -eq 0 ]] && [[ -n "$BT_DOC" && -f "$BT_DOC.sig" ]] && [[ "$BT_AUTH" -eq "$BT_PENDING" && "$BT_PENDING" -ge 2 ]] \
    && grep -q 'batch.recorded' <<<"$BT_OUT"; then
    printf 'ok    %-34s %s act(s), one batch signature\n' "a batch records every act" "$BT_AUTH"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, %s of %s authorized: %s\n' "a batch records every act" "$BT_STATUS" "$BT_AUTH" "$BT_PENDING" "$(grep -E '^(ERROR|WARN)' <<<"$BT_OUT" | head -3 | tr '\n' '|')"; FAILED=$((FAILED + 1))
fi

# `war check` believes each record because a verifying batch lists its exact
# response — not because the record says so.
BT_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
BT_SIGNED=$(grep -c 'PASS authority.signed .*BT-WAR-' <<<"$BT_CHECK")
if [[ "$BT_SIGNED" -ge 2 ]] && ! grep -q 'authority.unsigned .*BT-WAR-' <<<"$BT_CHECK"; then
    printf 'ok    %-34s %s record(s) signed through the batch\n' "the check believes the batch" "$BT_SIGNED"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "the check believes the batch" "$(grep -E 'authority\.' <<<"$BT_CHECK" | head -3 | tr '\n' '|')"; FAILED=$((FAILED + 1))
fi

BT_ATT=$("$WAR" --root "$PLANT_ROOT" attest --all --verify 2>&1)
BT_ATT_STATUS=$?
if [[ $BT_ATT_STATUS -eq 0 ]] && grep -q 'batch' <<<"$BT_ATT" && ls "$PLANT_ROOT"/docs/authority/batches/attestations/*.dsse.json >/dev/null 2>&1; then
    printf 'ok    %-34s one envelope over the batch verifies\n' "the batch is attested"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s: %s\n' "the batch is attested" "$BT_ATT_STATUS" "$(grep -E '^(ERROR|WARN)' <<<"$BT_ATT" | head -2 | tr '\n' '|')"; FAILED=$((FAILED + 1))
fi

# A response edited after the batch is no longer covered: the batch lists
# the old bytes, and the check says unsigned or invalid by name.
BT_RESP=$(ls "$PLANT_ROOT"/docs/authority/responses/BT-WAR-*.response.toml | head -1)
printf '\n# edited after the batch\n' >> "$BT_RESP"
BT_CHECK2=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
if grep -qE 'authority\.(unsigned|signature-invalid)' <<<"$BT_CHECK2"; then
    printf 'ok    %-34s the edited response is not covered\n' "a batch covers exact bytes only"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s an edited response still reads as signed\n' "a batch covers exact bytes only"; FAILED=$((FAILED + 1))
fi

ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$BT_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$BT_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
unset SSH_AGENT_PID
command rm -rf "$BT_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
