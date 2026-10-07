# shellcheck shell=bash
# M9: ordinary work is ordinary. What an agent meets in a repository that
# uses OpenWarrant says it may carry on, and a failed signature names what is
# missing and blocks only the sign-off.
#
# Each claim is paired with an observed refusal:
# - `war next` with nothing tracked says "nothing tracked; work freely"; with
#   tracked work none of which is ready, "nothing tracked is ready"; with a
#   ready item, neither.
# - In a scaffolded program with signatures pending, workable rows (ready
#   items, agent acts) come before every HUMAN row; the order predicate
#   refuses a planted HUMAN-first listing.
# - `war doctor` probes signing and signs nothing: the tree is unchanged, and
#   against a throwaway agent it tells "no key loaded" from "one key".
#   `--fix-signing` without a terminal is refused and writes nothing.
# - A `war sign` refusal names the missing file (roles.toml) or the key the
#   agent lacks, ends "This blocks only the sign-off, not your work.", and
#   carries a remedy; nothing is written.
# - A text newer than this `war` warns as version skew in `war prime` and
#   `war doctor`; the same stamp at this version says nothing.
# - The adopt Warrant's intent no longer puts the program's own work out of
#   scope.
# - `war eval ordinary` with fixture agents: one that fixes the bug passes;
#   one that refuses, one that asks for a Warrant and one that edits nothing
#   each fail by name. No model runs here.

echo "== ordinary work is ordinary (M9) =="
OW_TMP=$(mktemp -d)
ow_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ow_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# --- war next: the three empty states -------------------------------------
OW_PLAIN=$(mktemp -d)
git -C "$OW_PLAIN" init -q .
"$WAR" --root "$OW_PLAIN" init >/dev/null 2>&1
OW_N1=$("$WAR" --root "$OW_PLAIN" next 2>&1)
OW_J1=$("$WAR" --root "$OW_PLAIN" --json next 2>&1)
"$WAR" --root "$OW_PLAIN" create "Plant ticket" --item "Only item" >/dev/null 2>&1
OW_ITEM=$("$WAR" --root "$OW_PLAIN" --json ready 2>/dev/null | python3 -c '
import sys, json
r = json.load(sys.stdin)["result"]["ready"]
print(r[0]["ticket"] + "/" + r[0]["item"])' 2>/dev/null)
OW_N3=$("$WAR" --root "$OW_PLAIN" next 2>&1)
"$WAR" --root "$OW_PLAIN" claim "$OW_ITEM" --as plant-agent >/dev/null 2>&1
OW_N2=$("$WAR" --root "$OW_PLAIN" next 2>&1)
if [[ $(head -1 <<<"$OW_N1") == 'nothing tracked; work freely' ]] && grep -q 'war create' <<<"$OW_N1" \
    && grep -q '"idle": "nothing tracked; work freely"' <<<"$OW_J1" \
    && [[ $(head -1 <<<"$OW_N2") == 'nothing tracked is ready; work freely' ]]; then
    ow_ok "next with nothing ready frees work" "untracked, then tracked-but-claimed"
else
    ow_fail "next with nothing ready frees work" "$(head -1 <<<"$OW_N1") | $(head -1 <<<"$OW_N2")"
fi
if [[ -n $OW_ITEM ]] && grep -qF "war claim $OW_ITEM" <<<"$OW_N3" && ! grep -q 'work freely' <<<"$OW_N3"; then
    ow_ok "a ready item replaces the idle line" "the claim is listed, no work-freely line"
else
    ow_fail "a ready item replaces the idle line" "$(tr '\n' '|' <<<"$OW_N3")"
fi

# --- war next in a scaffolded program: workable rows first ----------------
OW_Q=$(scratch_queue OQ)
[[ -d "${OW_Q:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch queue (run through conformance/plant.sh)\n' >&2; exit 9; }
"$WAR" --root "$OW_Q" create "Plant work" --item "Ready plant item" >/dev/null 2>&1
OW_NQ=$("$WAR" --root "$OW_Q" next 2>&1)
# ow_workable_first <listing>: every `ready`/`agent` row before the first
# HUMAN row, and at least one of each.
ow_workable_first() {
    local first_human last_work
    first_human=$(grep -n -m1 '^HUMAN ' <<<"$1" | cut -d: -f1)
    last_work=$(grep -nE '^(ready|agent) ' <<<"$1" | tail -1 | cut -d: -f1)
    [[ -n $first_human && -n $last_work ]] && (( last_work < first_human ))
}
OW_BAD=$(printf 'HUMAN  OQ-WAR-0001  authorize  war sign OQ-WAR-0001\n%s\n' "$OW_NQ")
if ow_workable_first "$OW_NQ" && ! ow_workable_first "$OW_BAD"; then
    ow_ok "workable rows before HUMAN rows" "ready and agent rows first; a HUMAN-first listing refused"
else
    ow_fail "workable rows before HUMAN rows" "$(grep -E '^(ready|agent|HUMAN) ' <<<"$OW_NQ" | tr '\n' '|')"
fi
corpus_reset "$OW_Q"

# --- war doctor: signing probes that sign nothing ---------------------------
OW_BEFORE=$(git -C "$OW_Q" status --porcelain --untracked-files=all)
OW_D=$(env -u SSH_AUTH_SOCK "$WAR" --root "$OW_Q" --json doctor 2>/dev/null)
OW_AFTER=$(git -C "$OW_Q" status --porcelain --untracked-files=all)
if [[ "$OW_BEFORE" == "$OW_AFTER" ]] && python3 -c '
import sys, json
v = json.loads(sys.argv[1])
s = v["result"]["signing"]
assert s["signed_anything"] is False and s["agent"] == "no-socket", s
assert v["result"]["ordinary_work_needs_authorization"] is False
d = [x for x in v["diagnostics"] if x["rule"] == "doctor.signing-agent"]
assert d and "SSH_AUTH_SOCK is not set" in d[0]["message"], d
assert d[0]["remedy"]["argv"] == ["war", "doctor", "--fix-signing"], d
' "$OW_D" 2>/dev/null; then
    ow_ok "doctor probes signing, signs nothing" "no agent named; tree unchanged"
else
    ow_fail "doctor probes signing, signs nothing" "tree moved, or the signing section is wrong"
fi
# A throwaway agent, asserted to hold only the throwaway key once loaded.
OW_SOCK="$OW_TMP/agent.sock"
OW_AGENT_PID=$(ssh-agent -a "$OW_SOCK" -s 2>/dev/null | sed -n 's/^SSH_AGENT_PID=\([0-9]*\);.*/\1/p')
ssh-keygen -q -t ed25519 -N "" -C plant-loaded -f "$OW_TMP/k1"
ssh-keygen -q -t ed25519 -N "" -C plant-other -f "$OW_TMP/k2"
OW_D0=$(SSH_AUTH_SOCK="$OW_SOCK" "$WAR" --root "$OW_Q" doctor 2>&1)
SSH_AUTH_SOCK="$OW_SOCK" ssh-add -q "$OW_TMP/k1" 2>/dev/null
OW_HELD=$(SSH_AUTH_SOCK="$OW_SOCK" ssh-add -L 2>/dev/null)
OW_D1=$(SSH_AUTH_SOCK="$OW_SOCK" "$WAR" --root "$OW_Q" doctor 2>&1)
if [[ $(wc -l <<<"$OW_HELD") -eq 1 ]] && [[ $(cut -d' ' -f2 <<<"$OW_HELD") == $(cut -d' ' -f2 "$OW_TMP/k1.pub") ]] \
    && grep -q 'doctor.signing-keys .*no key loaded' <<<"$OW_D0" \
    && grep -q 'doctor.signing-keys .*holds 1 key(s)' <<<"$OW_D1"; then
    ow_ok "doctor reads the agent's keys" "no key loaded, then the one throwaway key"
else
    ow_fail "doctor reads the agent's keys" "held $(wc -l <<<"$OW_HELD"); $(grep 'signing-keys' <<<"$OW_D0$OW_D1" | tr '\n' '|')"
fi
OW_FIX=$(env -u SSH_AUTH_SOCK "$WAR" --root "$OW_Q" doctor --fix-signing </dev/null 2>&1)
OW_FIX_RC=$?
if [[ $OW_FIX_RC -ne 0 ]] && grep -q 'doctor.fix-needs-tty' <<<"$OW_FIX" \
    && [[ -z $(git -C "$OW_Q" status --porcelain --untracked-files=all) ]]; then
    ow_ok "--fix-signing needs a terminal" "refused (exit $OW_FIX_RC), nothing written"
else
    ow_fail "--fix-signing needs a terminal" "exit $OW_FIX_RC: $(tr '\n' '|' <<<"$OW_FIX")"
fi

# --- war sign: a refusal names what is missing ------------------------------
# The key allowed_signers names (k2) is not the one the agent holds (k1).
printf 'planthuman namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$OW_TMP/k2.pub")" \
    > "$OW_Q/docs/authority/allowed_signers"
cat > "$OW_Q/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Human"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/101-ordinary-work.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "planthuman"
ROLES
OW_S=$(SSH_AUTH_SOCK="$OW_SOCK" "$WAR" --root "$OW_Q" sign OQ-WAR-0001 --ssh-sign --as "Plant Human" 2>&1)
OW_S_RC=$?
OW_RESP=$(find "$OW_Q/docs/authority/responses" -name '*.response.toml' 2>/dev/null | head -1)
if [[ $OW_S_RC -ne 0 && -z $OW_RESP ]] && grep -q 'sign.ssh-refused .*none of them the ssh-ed25519' <<<"$OW_S" \
    && grep -q 'This blocks only the sign-off, not your work.' <<<"$OW_S" \
    && grep -q '→ info: war doctor' <<<"$OW_S"; then
    ow_ok "a signing refusal names the key" "the agent lacks k2; remedy war doctor; nothing written"
else
    ow_fail "a signing refusal names the key" "exit $OW_S_RC; response $OW_RESP; $(grep -E '^ERROR|→ info' <<<"$OW_S" | tr '\n' '|')"
fi
SSH_AGENT_PID="$OW_AGENT_PID" ssh-agent -k >/dev/null 2>&1 || kill "$OW_AGENT_PID" 2>/dev/null
corpus_reset "$OW_Q"
# With no roles.toml at all, the refusal names the file.
command rm -f "$OW_Q/docs/authority/roles.toml"
OW_W=$(env -u SSH_AUTH_SOCK "$WAR" --root "$OW_Q" sign OQ-WAR-0001 --ssh-sign 2>&1)
OW_W_RC=$?
if [[ $OW_W_RC -ne 0 ]] && grep -q 'sign.who .*docs/authority/roles.toml does not exist' <<<"$OW_W" \
    && grep -q 'This blocks only the sign-off, not your work.' <<<"$OW_W" \
    && grep -q '→ info: war doctor' <<<"$OW_W"; then
    ow_ok "no roles.toml is named as missing" "sign.who names the file and how to make it"
else
    ow_fail "no roles.toml is named as missing" "exit $OW_W_RC: $(grep -E '^ERROR' <<<"$OW_W" | head -2 | tr '\n' '|')"
fi
corpus_reset "$OW_Q"

# --- version skew --------------------------------------------------------------
OW_VER=$("$WAR" --version 2>/dev/null | awk '{print $2}')
sed -i 's/written by war [^ ]* -->/written by war 99.0.0 -->/' "$OW_PLAIN/AGENTS.md"
OW_P=$("$WAR" --root "$OW_PLAIN" prime 2>&1 >/dev/null)
OW_DS=$(env -u SSH_AUTH_SOCK "$WAR" --root "$OW_PLAIN" doctor 2>&1)
sed -i "s/written by war 99.0.0 -->/written by war $OW_VER -->/" "$OW_PLAIN/AGENTS.md"
OW_P2=$("$WAR" --root "$OW_PLAIN" prime 2>&1 >/dev/null)
if grep -q 'warning (install.version-skew): AGENTS.md expects war 99.0.0' <<<"$OW_P" \
    && grep -q 'install.version-skew' <<<"$OW_DS" && [[ -n $OW_VER ]] \
    && ! grep -q 'version-skew' <<<"$OW_P2"; then
    ow_ok "newer text warns as version skew" "prime and doctor at 99.0.0; silent at $OW_VER"
else
    ow_fail "newer text warns as version skew" "prime: $OW_P | at $OW_VER: $OW_P2"
fi

# --- the adopt Warrant's intent -------------------------------------------------
OW_INTENT=$(cat "$OW_Q"/docs/warrants/OQ-WAR-0001/atoms/10-intent.md 2>/dev/null)
if grep -q 'without waiting for it' <<<"$OW_INTENT" \
    && ! grep -q "The program's own work" <<<"$OW_INTENT"; then
    ow_ok "adopt intent frees the program's work" "other work goes ahead without it"
else
    ow_fail "adopt intent frees the program's work" "$(tail -4 <<<"$OW_INTENT" | tr '\n' '|')"
fi
corpus_gone "$OW_Q"

# --- war eval ordinary, with fixture agents (no model) -------------------------
OW_FX="$PWD/evals/ordinary/fixtures"
ow_eval() { "$WAR" eval ordinary --agent bash --agent "$OW_FX/$1.sh" 2>&1; }
OW_E=$(ow_eval fixes); OW_E_RC=$?
if [[ $OW_E_RC -eq 0 ]] && grep -q 'eval.ordinary-ok' <<<"$OW_E"; then
    ow_ok "an agent that fixes the bug passes" "eval.ordinary-ok"
else
    ow_fail "an agent that fixes the bug passes" "exit $OW_E_RC: $(grep -E '^(ERROR|UNKNOWN)' <<<"$OW_E" | tr '\n' '|')"
fi
OW_FAILS=""
for pair in refuses:eval.ordinary-refused asks-for-warrant:eval.ordinary-asked-for-warrant \
    edits-nothing:eval.ordinary-no-edit; do
    OW_E=$(ow_eval "${pair%%:*}"); OW_E_RC=$?
    if [[ $OW_E_RC -ne 2 ]] || ! grep -q "^ERROR ${pair#*:} " <<<"$OW_E"; then
        OW_FAILS+="${pair%%:*} (exit $OW_E_RC) "
    fi
done
OW_E=$("$WAR" eval ordinary 2>&1); OW_E_RC=$?
if [[ -z $OW_FAILS && $OW_E_RC -eq 1 ]] && grep -q 'eval.no-agent' <<<"$OW_E"; then
    ow_ok "refusing agents fail by name" "refused, asked-for-warrant, no-edit; no agent is eval.no-agent"
else
    ow_fail "refusing agents fail by name" "not caught: ${OW_FAILS:-none}; no-agent exit $OW_E_RC"
fi

command rm -rf "$OW_TMP" "$OW_PLAIN"
unset OW_TMP OW_PLAIN OW_N1 OW_N2 OW_N3 OW_J1 OW_ITEM OW_Q OW_NQ OW_BAD OW_BEFORE OW_AFTER OW_D \
    OW_SOCK OW_AGENT_PID OW_D0 OW_D1 OW_HELD OW_FIX OW_FIX_RC OW_S OW_S_RC OW_RESP OW_W OW_W_RC \
    OW_VER OW_P OW_DS OW_P2 OW_INTENT OW_FX OW_E OW_E_RC OW_FAILS
unset -f ow_ok ow_fail ow_workable_first ow_eval
