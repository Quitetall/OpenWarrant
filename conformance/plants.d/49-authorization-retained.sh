# shellcheck shell=bash
# OW-WAR-0144 — amending an authorization keeps the superseded one beside it.
#
# A scratch program, a key generated here and a throwaway ssh-agent holding
# only it (as 96-batch.sh). Revision 1 is authorized, the contract amended,
# revision 2 authorized; the revision-1 bytes must survive beside the new
# record so the revision-1 attestation still verifies.

echo "== authorization retention (OW-WAR-0144) =="
PLANT_ROOT=$(scratch_corpus AR)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
AR_TMP=$(mktemp -d)
AR_W="$PLANT_ROOT/docs/warrants/AR-WAR-0001"
ssh-keygen -q -t ed25519 -N "" -C plant -f "$AR_TMP/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$AR_TMP/id_plant.pub")" > "$PLANT_ROOT/docs/authority/allowed_signers"
cat > "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/49-authorization-retained.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while these plants run."
ssh_principal = "plant"
ROLES
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register a plant signer" >/dev/null 2>&1
AR_OLD_SOCK=${SSH_AUTH_SOCK:-}
eval "$(ssh-agent -s)" >/dev/null
ssh-add -q "$AR_TMP/id_plant" 2>/dev/null
AR_WANT=$(ssh-keygen -lf "$AR_TMP/id_plant.pub" | awk '{print $2}')
[[ "$(ssh-add -l | awk '{print $2}')" == "$AR_WANT" ]] || { printf 'PLANT SETUP FAILED: the agent holds another key\n' >&2; ssh-agent -k >/dev/null; exit 9; }
ar_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ar_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ar_sign() { "$WAR" --root "$PLANT_ROOT" sign AR-WAR-0001 --ssh-sign --as "Plant Signer" "$@" </dev/null 2>&1; }

# Revision 1, then an amendment and revision 2.
ar_sign >/dev/null
AR_R1=$(sha256sum "$AR_W/authorization.toml" 2>/dev/null | cut -c1-64)
printf '\nAmended by the retention plant.\n' >> "$AR_W/atoms/10-intent.md"
mkdir -p "$AR_W/amendments"
cat > "$AR_W/amendments/AM-001.yaml" <<'AM'
schema: "oh.war/amendment/v1"

id: "AM-001"
band: "manual_revision"
reason: "The retention plant amends the intent to make a revision 2."
governing_adr_or_policy: "sas://WAR-SAS-31"
artifact_admissibility: "remain_admissible"
restart_or_repair_instruction: "Continue."
re_preflight_required: "false"
authorizer: "Plant Signer"
effective_time: "2026-09-24"

semantic_diff:
  - element: "intent"
    before: "The scaffold's intent."
    after: "The same, with one line added."

affected_stages: []
affected_milestones: []
AM
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1

# OBL-002 first, while revision 2 is still pending: the dry run keeps
# nothing, and a colliding file under the kept name refuses the act.
ar_dir() { (cd "$AR_W" && find . -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum); }
AR_BEFORE=$(ar_dir)
"$WAR" --root "$PLANT_ROOT" sign AR-WAR-0001 --as "Plant Signer" --dry-run >/dev/null 2>&1
if [[ -n "$AR_R1" && "$(ar_dir)" == "$AR_BEFORE" ]]; then
    ar_ok "a dry run keeps nothing" "the Warrant directory is byte-identical"
else
    ar_fail "a dry run keeps nothing" "revision 1 ${AR_R1:-was not recorded}; directory moved"
fi
AR_KEPT="$AR_W/authorization.${AR_R1:0:8}.toml"
printf 'not the authorization\n' > "$AR_KEPT"
AR_BEFORE=$(ar_dir)
AR_OUT=$(ar_sign); AR_STATUS=$?
if [[ $AR_STATUS -ne 0 ]] && grep -q 'authorize.retire-collision' <<<"$AR_OUT" \
    && [[ "$(sha256sum "$AR_W/authorization.toml" | cut -c1-64)" == "$AR_R1" ]]; then
    ar_ok "a colliding kept name refuses" "authorize.retire-collision, revision 1 untouched"
else
    ar_fail "a colliding kept name refuses" "exit $AR_STATUS: $(grep -E '^(ERROR|PASS authorize)' <<<"$AR_OUT" | head -2 | tr '\n' '|')"
fi
command rm -f "$AR_KEPT"
command rm -f "$PLANT_ROOT"/docs/authority/responses/AR-WAR-0001.draft.toml

# OBL-001: revision 2 recorded, revision 1 kept beside it byte for byte.
AR_OUT=$(ar_sign); AR_STATUS=$?
if [[ $AR_STATUS -eq 0 ]] && [[ -f "$AR_KEPT" && "$(sha256sum "$AR_KEPT" | cut -c1-64)" == "$AR_R1" ]] \
    && grep -q 'revision = 2' "$AR_W/authorization.toml"; then
    ar_ok "revision 1 is kept beside revision 2" "authorization.${AR_R1:0:8}.toml, same bytes"
else
    ar_fail "revision 1 is kept beside revision 2" "exit $AR_STATUS: $(grep -E '^ERROR' <<<"$AR_OUT" | head -2 | tr '\n' '|')"
fi
AR_ATT=$("$WAR" --root "$PLANT_ROOT" attest --all --verify 2>&1); AR_ATT_STATUS=$?
if [[ $AR_ATT_STATUS -eq 0 ]] && grep -q 'attest.subject-archived' <<<"$AR_ATT"; then
    ar_ok "the revision-1 envelope verifies" "attest.subject-archived"
else
    ar_fail "the revision-1 envelope verifies" "exit $AR_ATT_STATUS: $(grep -E '^ERROR' <<<"$AR_ATT" | head -2 | tr '\n' '|')"
fi
# Control: without the kept file the same verify reports drift, so the
# retention is what makes it pass.
command mv "$AR_KEPT" "$AR_TMP/kept"
AR_ATT=$("$WAR" --root "$PLANT_ROOT" attest --all --verify 2>&1); AR_ATT_STATUS=$?
command mv "$AR_TMP/kept" "$AR_KEPT"
if [[ $AR_ATT_STATUS -ne 0 ]] && grep -q 'attest.subject-drift .*authorization.toml' <<<"$AR_ATT"; then
    ar_ok "without it, the envelope drifts" "attest.subject-drift names authorization.toml"
else
    ar_fail "without it, the envelope drifts" "exit $AR_ATT_STATUS"
fi

# OBL-003: the kept file is not a second authorization.
AR_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
AR_LIST=$("$WAR" --root "$PLANT_ROOT" sign --list 2>&1)
if ! grep -q '^ERROR' <<<"$AR_CHECK" && ! grep -q 'AR-WAR-0001 .*authorize' <<<"$AR_LIST"; then
    ar_ok "the kept file is not a record" "check clean; no authorization pending"
else
    ar_fail "the kept file is not a record" "$(grep -E '^ERROR' <<<"$AR_CHECK" | head -2 | tr '\n' '|') $(grep 'AR-WAR-0001' <<<"$AR_LIST" | head -1)"
fi

ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$AR_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$AR_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
unset SSH_AGENT_PID
command rm -rf "$AR_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
