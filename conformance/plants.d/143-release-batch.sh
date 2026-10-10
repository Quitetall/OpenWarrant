# shellcheck shell=bash
# OW-WAR-0148 M14 — signing batched at release: `war sign release <tag>`
# (docs/PRESETS.md; the batch is OW-WAR-0072's).
#
# A scratch program whose queue holds three acts (scratch_queue: two
# authorizations and a SAS acceptance), switched to the team preset. The
# signer's key is generated here and held by a throwaway ssh-agent asserted
# to hold only it. Each claim is paired with a refusal:
#
#   1. the request lists exactly the acts awaiting a signature, and the one
#      batch carries exactly the two authorizations; the SAS acceptance is
#      named as signing alone. It writes nothing and asks no key. Refused: a
#      tag that is not one word (release.tag), and, under the vibe preset,
#      nothing is asked (release.nothing-asked).
#   2. `--ssh-sign` signs that batch once; both authorizations are recorded
#      and verify, each response naming the release. Asked again, the
#      request lists only the acceptance, and the batch is refused empty
#      (batch.empty).

echo "== signing batched at release (M14) =="
PLANT_ROOT=$(scratch_queue RB)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
RB_T=$(mktemp -d)
rb_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rb_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
rbw() { "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
rb_py() { python3 -c "import json, sys; v = json.load(sys.stdin); print($1)" 2>/dev/null; }
rbw admin preset team >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qam "team preset" >/dev/null 2>&1
RB_SIGNER="your-name-here"
RB_LIST=$(rbw sign --list 2>&1)
RB_WANT=$(grep -E '^  ' <<<"$RB_LIST" | sed 's/^  //' | sort)

# ---- 1. the request -------------------------------------------------------------------
RB_REQ_JSON=$(rbw --json sign release v0.1.0 --as "$RB_SIGNER")
RB_REQ_RC=$?
RB_PENDING=$(rb_py '"\n".join(sorted(v["result"]["pending"]))' <<<"$RB_REQ_JSON")
RB_BATCH=$(rb_py 'v["result"]["batch"]' <<<"$RB_REQ_JSON")
RB_LEFT=$(rb_py '"|".join(v["result"]["left_out"])' <<<"$RB_REQ_JSON")
RB_DIRTY=$(git -C "$PLANT_ROOT" status --porcelain)
if [[ $RB_REQ_RC -eq 0 && -n "$RB_WANT" && "$RB_PENDING" == "$RB_WANT" && -z "$RB_DIRTY" ]] \
    && [[ "$RB_BATCH" == "2 act(s) as $RB_SIGNER (authorizer), one signature: authorize RB-WAR-0001, authorize RB-WAR-0002" ]] \
    && grep -qF 'a SAS acceptance signs alone' <<<"$RB_LEFT"; then
    rb_ok "the request lists exactly the acts" "$(wc -l <<<"$RB_PENDING") awaiting; one batch: the two authorizations; the SAS acceptance alone; nothing written"
else
    rb_fail "the request lists exactly the acts" "exit $RB_REQ_RC; pending '$(tr '\n' '|' <<<"$RB_PENDING")' want '$(tr '\n' '|' <<<"$RB_WANT")'; batch '$RB_BATCH'; dirty '$RB_DIRTY'"
fi
RB_TAG=$(rbw sign release "v 1" 2>&1); RB_TAG_RC=$?
cp "$PLANT_ROOT/openwarrant.toml" "$RB_T/team.toml"
rbw admin preset vibe >/dev/null 2>&1
RB_VIBE=$(rbw --json sign release v0.1.0 | rb_py '"%s %s %s" % (v["exit_code"], ",".join(d["rule"] for d in v["diagnostics"]), len(v["result"]["pending"]))')
cp "$RB_T/team.toml" "$PLANT_ROOT/openwarrant.toml"
if [[ $RB_TAG_RC -eq 2 && "$RB_VIBE" == "0 release.nothing-asked 3" ]] && grep -qF 'release.tag' <<<"$RB_TAG"; then
    rb_ok "a bad tag refused; vibe asks none" "release.tag; under vibe release.nothing-asked, the three acts still named"
else
    rb_fail "a bad tag refused; vibe asks none" "tag exit $RB_TAG_RC: $(head -1 <<<"$RB_TAG"); vibe '$RB_VIBE'"
fi

# ---- 2. the one signature -------------------------------------------------------------------
ssh-keygen -q -t ed25519 -N "" -C plant-release -f "$RB_T/id_plant"
RB_PUB=$(cut -d' ' -f1,2 "$RB_T/id_plant.pub")
RB_PRINCIPAL=$(awk -F'"' '/^actor = "your-name-here"/{f=1} f && /^ssh_principal/{print $2; exit}' "$PLANT_ROOT/docs/authority/roles.toml")
printf '%s namespaces="oh.war/response,oh.war/dsse" %s\n' "$RB_PRINCIPAL" "$RB_PUB" > "$PLANT_ROOT/docs/authority/allowed_signers"
eval "$(ssh-agent -s > "$RB_T/agent.env"; cat "$RB_T/agent.env")" >/dev/null
ssh-add -q "$RB_T/id_plant" 2>/dev/null
RB_KEYS=$(ssh-add -L 2>/dev/null)
if [[ -n "$RB_PRINCIPAL" && "$(wc -l <<<"$RB_KEYS")" == "1" && "$RB_KEYS" == "$RB_PUB"* ]]; then
    RB_SIGN=$(rbw sign release v0.1.0 --as "$RB_SIGNER" --ssh-sign 2>&1); RB_SIGN_RC=$?
else
    RB_SIGN="the throwaway agent holds more than the plant's key, or no principal: $RB_KEYS"; RB_SIGN_RC=9
fi
ssh-agent -k >/dev/null 2>&1 || true
unset SSH_AUTH_SOCK SSH_AGENT_PID
RB_BATCHES=$(ls "$PLANT_ROOT/docs/authority/batches/" 2>/dev/null | grep -c '\.json$')
RB_MEANING=$(grep -l 'Signed at release v0.1.0.' "$PLANT_ROOT"/docs/authority/responses/RB-WAR-000*.response.toml 2>/dev/null | wc -l)
RB_CHECK=$(rbw check 2>&1)
RB_AGAIN=$(rbw --json sign release v0.1.0 --as "$RB_SIGNER")
RB_AGAIN_RC=$?
RB_AGAIN_PENDING=$(rb_py '"|".join(v["result"]["pending"])' <<<"$RB_AGAIN")
RB_AGAIN_RULES=$(rb_py '",".join(d["rule"] for d in v["diagnostics"])' <<<"$RB_AGAIN")
if [[ $RB_SIGN_RC -eq 0 && "$RB_BATCHES" == "1" && "$RB_MEANING" == "2" ]] \
    && line_has -F 'RB-WAR-0001' -F 'authority.signed' <<<"$RB_CHECK" \
    && line_has -F 'RB-WAR-0002' -F 'authority.signed' <<<"$RB_CHECK"; then
    rb_ok "one signature signs the batch" "one batch document; both authorizations verify; each response names release v0.1.0"
else
    rb_fail "one signature signs the batch" "exit $RB_SIGN_RC, batches $RB_BATCHES, meanings $RB_MEANING: $(grep -E '^(ERROR|UNKNOWN)' <<<"$RB_SIGN" | head -2 | tr '\n' '|')"
fi
if [[ $RB_AGAIN_RC -eq 2 && "$RB_AGAIN_PENDING" == "SAS 0.1.0"* && "$RB_AGAIN_PENDING" != *"|"* ]] \
    && [[ "$RB_AGAIN_RULES" == *"batch.empty"* ]]; then
    rb_ok "signed acts leave the request" "asked again: only the SAS acceptance waits; batch.empty"
else
    rb_fail "signed acts leave the request" "exit $RB_AGAIN_RC; pending '$RB_AGAIN_PENDING'; rules '$RB_AGAIN_RULES'"
fi

unset -f rb_ok rb_fail rbw rb_py
command rm -rf "$RB_T"
corpus_gone "$PLANT_ROOT"
