# shellcheck shell=bash
# One format, two layouts (docs/design/one-format.md): the formal path over a
# single-file Warrant, and the directory Warrant beside it unchanged.
#
# A scratch program, a throwaway key that `ssh-keygen -Y sign -f` uses
# directly (no agent, SSH_AUTH_SOCK unset), and a register naming one person
# and one agent. Each claim beside the input it refuses:
#   a single-file Warrant is authorized by a person; an agent's authorization is refused;
#   an edit after authorization moves the contract, and resolve refuses the old signature;
#   evidence, an independent verifier fixture and a person's resolution close it;
#   an agent cannot resolve it; the performer cannot verify it;
#   the old directory Warrant still goes the same way.

echo "== one-file formal path (one-format) =="
SF_ROOT=$(scratch_corpus SF)
[[ -d "${SF_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
SF_TMP=$(mktemp -d)
SF_RESP="$SF_ROOT/docs/authority/responses"
sf()        { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID \
                  -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
                  PATH="$REPO_ROOT/target/debug:$PATH" "$REPO_ROOT/${WAR#./}" --root "$SF_ROOT" "$@"; }
sf_commit() {
    git -C "$SF_ROOT" add -A >/dev/null 2>&1
    git -C "$SF_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgsign=false \
        commit -qm "$1" >/dev/null 2>&1
}
sf_sign()   { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID ssh-keygen -q -Y sign -f "$SF_TMP/id_plant" -n oh.war/response "$1" >/dev/null 2>&1; }
sf_setup_failed() { printf 'PLANT SETUP FAILED: %s in %s\n' "$1" "$SF_ROOT" >&2; /usr/bin/rm -rf "$SF_TMP"; exit 9; }
sf_ok()     { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sf_fail()   { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sf_lines()  { grep -E '^(PASS|ERROR|UNKNOWN|WARN)' <<<"$1" | head -3 | tr '\n' '|'; }
sf_digest() { sf authorize "$1" 2>/dev/null | grep '^contract_digest' | cut -d'"' -f2; }
sf_set()    { sf authorize "$1" 2>/dev/null | grep '^deliverable_set_digest' | cut -d'"' -f2; }
# sf_auth <alias> <authorizer> <file>: an authorization response over the contract as it stands.
sf_auth() {
    printf 'schema = "oh.war/authorization-response/v1"\nwarrant = "%s"\ncontract_digest = "%s"\nauthorizer = "%s"\nacting_role = "authorizer"\nmeaning = "plant fixture"\neffective_time = "2026-09-10T00:00:00Z"\nindependence = "separate_role"\ndeliverable_set_digest = "%s"\n' \
        "$1" "$(sf_digest "$1")" "$2" "$(sf_set "$1")" > "$3"
}
# sf_resolution <alias> <resolver> <file>
sf_resolution() {
    printf 'schema = "oh.war/resolution-response/v1"\nwarrant = "%s"\ncontract_digest = "%s"\nresolved_by = "%s"\nacting_role = "resolver"\ncommon_outcome = "satisfied"\nprofile_outcome = "delivered"\nmeaning = "plant fixture"\neffective_time = "2026-09-22T00:00:00Z"\n' \
        "$1" "$(sf_digest "$1")" "$2" > "$3"
}

env -u SSH_AUTH_SOCK -u SSH_AGENT_PID ssh-keygen -q -t ed25519 -N "" -C plant -f "$SF_TMP/id_plant" \
    || sf_setup_failed "ssh-keygen"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$SF_TMP/id_plant.pub")" \
    > "$SF_ROOT/docs/authority/allowed_signers"
cat > "$SF_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Human"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/200-one-file-formal.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs; its key is generated and discarded here."
ssh_principal = "plant"

[[assignment]]
actor = "claude"
actor_kind = "agent"
roles = ["performer", "authorizer", "resolver"]
assigned_by = "conformance/plants.d/200-one-file-formal.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "The performer. The roles it is given here are the ones it must still be refused."
ROLES
mkdir -p "$SF_RESP"
sf sas propose 0.1.0 >/dev/null 2>&1 || sf_setup_failed "war sas propose"
SF_SAS=$(grep '^sha256' "$SF_ROOT/docs/sas/revisions/0.1.0.toml" 2>/dev/null | cut -d'"' -f2)
printf 'schema = "oh.war/sas-acceptance-response/v1"\nversion = "0.1.0"\nsha256 = "%s"\naccepted_by = "Plant Human"\nacting_role = "owner"\nmeaning = "plant fixture"\neffective_time = "2026-09-02T00:00:00Z"\n' \
    "$SF_SAS" > "$SF_RESP/SAS-0.1.0.response.toml"
sf_sign "$SF_RESP/SAS-0.1.0.response.toml"
sf sas accept 0.1.0 --response "$SF_RESP/SAS-0.1.0.response.toml" >/dev/null 2>&1 || sf_setup_failed "war sas accept"
SF_VERIFIER="$REPO_ROOT/conformance/fixtures/verifier/establishes-all.sh"
sed 's/fixture-verifier/claude/' "$SF_VERIFIER" > "$SF_TMP/self.sh" && chmod +x "$SF_TMP/self.sh"
python3 - "$SF_ROOT/openwarrant.toml" "$SF_VERIFIER" <<'PY' || sf_setup_failed "the verifier fixture"
import sys
path, verifier = sys.argv[1], sys.argv[2]
text = open(path).read()
if "verifier_argv = []" not in text:
    sys.exit(1)
open(path, "w").write(text.replace("verifier_argv = []", f'verifier_argv = ["{verifier}"]', 1))
PY

SF_NEW=$(sf --json plan new "A note is delivered" --format 1.0 2>/dev/null)
SF_A=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["alias"])' <<<"$SF_NEW" 2>/dev/null)
SF_DOC="$SF_ROOT/docs/warrants/$SF_A.md"
[[ -n "$SF_A" && -f "$SF_DOC" ]] || sf_setup_failed "plan new --format 1.0: $SF_NEW"
python3 - "$SF_DOC" <<'PY' || sf_setup_failed "the obligation"
import sys
p = sys.argv[1]
t = open(p).read()
head, sep, _ = t.partition("## OBL-001")
open(p, "w").write(head + sep + " \u00b7 obligation \u2014 the note is delivered and the record checks\n"
    "scope: NOTE.md and this Warrant's record, nothing else.\n"
    "evidence: war check over the program passes.\n"
    "refusal: an edit to this document after it is authorized.\n"
    "gate: gate://software.repo.war-check@1.0.0\n")
PY
SF_STATE="$SF_ROOT/docs/warrants/$SF_A"
SF_D="SF-WAR-0001"
SF_DDIR="$SF_ROOT/docs/warrants/$SF_D"
mkdir -p "$SF_STATE"
printf 'delivered\n' > "$SF_ROOT/NOTE.md"
printf 'adopted\n' > "$SF_ROOT/ADOPTED.md"
for sf_pair in "$SF_STATE:NOTE.md:\"OBL-001\"" "$SF_DDIR:ADOPTED.md:\"OBL-001\", \"OBL-002\""; do
    sf_dir=${sf_pair%%:*}; sf_rest=${sf_pair#*:}; sf_file=${sf_rest%%:*}
    cat > "$sf_dir/deliverables.toml" <<TOML
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "The delivered note"
kind = "file"
target_ref = "$sf_file"
required = true
content_addressed = true
provenance_required = true
obligation_refs = [${sf_rest#*:}]

[deliverable.provenance]
producer = "conformance/plants.d/200-one-file-formal.sh"
producing_attempt = "$(basename "$sf_dir")/attempt-1"
contract_digest = "unrecorded"
input_digests = []
tool_or_runtime_identity = "printf"
creation_method = "generated"
content_digest = "sha256:$(sha256sum < "$SF_ROOT/$sf_file" | cut -d' ' -f1)"
media_type = "text/markdown"
classification = "internal"
retention = "repository-lifetime"
source_holder = "git"
TOML
    printf 'schema = "oh.war/rationale/v1"\n' > "$sf_dir/rationale.toml"
done
sf compile >/dev/null 2>&1; sf_commit "two Warrants, one file and one directory"

# The document is a Warrant every act reads: check passes it, and a request names its contract.
SF_CHECK=$(sf check "$SF_A" 2>&1); SF_CHECK_RC=$?
SF_C1=$(sf_digest "$SF_A")
if [[ $SF_CHECK_RC -eq 0 && "$SF_C1" =~ ^[0-9a-f]{64}$ ]]; then
    sf_ok "a single file has a contract" "war check $SF_A passes; contract ${SF_C1:0:12}…"
else
    sf_fail "a single file has a contract" "check $SF_CHECK_RC, digest '$SF_C1': $(sf_lines "$SF_CHECK")"
fi

# An agent's authorization is refused, and nothing is written.
sf_auth "$SF_A" claude "$SF_RESP/$SF_A.agent.response.toml"
SF_OUT=$(sf authorize "$SF_A" --response "$SF_RESP/$SF_A.agent.response.toml" 2>&1); SF_RC=$?
if [[ $SF_RC -ne 0 && ! -f "$SF_STATE/authorization.toml" ]] && grep -q '^ERROR authorize.not-permitted' <<<"$SF_OUT"; then
    sf_ok "an agent cannot authorize it" "$(grep -m1 -oE '^ERROR +[a-z.-]+' <<<"$SF_OUT" | tr -s ' '); nothing written"
else
    sf_fail "an agent cannot authorize it" "exit $SF_RC: $(sf_lines "$SF_OUT")"
fi
/usr/bin/rm -f "$SF_RESP/$SF_A.agent.response.toml"

# A person's signed authorization is recorded beside the document.
for sf_w in "$SF_A" "$SF_D"; do
    sf_auth "$sf_w" "Plant Human" "$SF_RESP/$sf_w.response.toml"
    sf_sign "$SF_RESP/$sf_w.response.toml"
    SF_OUT=$(sf authorize "$sf_w" --response "$SF_RESP/$sf_w.response.toml" 2>&1); SF_RC=$?
    if [[ $SF_RC -eq 0 ]] && grep -q '^PASS authorize.recorded' <<<"$SF_OUT"; then
        sf_ok "a person authorizes $sf_w" "authorize.recorded"
    else
        sf_fail "a person authorizes $sf_w" "exit $SF_RC: $(sf_lines "$SF_OUT")"
    fi
done
[[ -f "$SF_STATE/authorization.toml" && ! -f "$SF_STATE/warrant.md" ]] \
    || sf_fail "state beside the document" "no $SF_A/authorization.toml, or the document moved"
sf compile >/dev/null 2>&1; sf_commit "authorized"

# An edit after authorization moves the contract: the signature no longer
# matches what the document says, and resolve refuses it.
printf '\nOne more sentence nobody signed.\n' >> "$SF_DOC"
SF_C2=$(sf_digest "$SF_A")
SF_DRY=$(sf resolve --dry-run "$SF_A" 2>&1)
if [[ -n "$SF_C2" && "$SF_C2" != "$SF_C1" ]] && grep -qiE 'authoriz' <<<"$SF_DRY" \
    && ! grep -q "^PASS" <<<"$(grep -iE 'authoriz' <<<"$SF_DRY")"; then
    sf_ok "an edit lapses the authorization" "contract ${SF_C1:0:8}… → ${SF_C2:0:8}…; resolve --dry-run blocks on authorization"
else
    sf_fail "an edit lapses the authorization" "$SF_C1 → $SF_C2: $(grep -iE 'authoriz' <<<"$SF_DRY" | head -3 | tr '\n' '|')"
fi
git -C "$SF_ROOT" checkout -q -- "docs/warrants/$SF_A.md"
[[ "$(sf_digest "$SF_A")" == "$SF_C1" ]] || sf_fail "the edit reverts" "contract did not return to $SF_C1"

# Evidence, an independent verdict, then resolution: an agent is refused, a person closes it.
for sf_w in "$SF_A" "$SF_D"; do
    SF_EV=$(sf evidence record "$sf_w" 2>&1) || sf_fail "evidence for $sf_w" "$(tail -5 <<<"$SF_EV" | tr '\n' '|')"
    sf compile >/dev/null 2>&1; sf_commit "evidence $sf_w"
    # The performer as its own verifier: the same fixture, naming claude.
    sed -i "s#$SF_VERIFIER#$SF_TMP/self.sh#" "$SF_ROOT/openwarrant.toml"
    SF_SELF=$(sf verify "$sf_w" --performer claude --run 2>&1); SF_SELF_RC=$?
    SF_SELF_N=$(find "$SF_ROOT/docs/warrants/$sf_w/verifications" -type f ! -name "bundle-*" 2>/dev/null | grep -c .)
    sed -i "s#$SF_TMP/self.sh#$SF_VERIFIER#" "$SF_ROOT/openwarrant.toml"
    git -C "$SF_ROOT" checkout -q -- . 2>/dev/null; git -C "$SF_ROOT" clean -qfd -- "docs/warrants/$sf_w" 2>/dev/null
    SF_IND=$(sf verify "$sf_w" --performer claude --run 2>&1)
    SF_IND_N=$(find "$SF_ROOT/docs/warrants/$sf_w/verifications" -type f ! -name "bundle-*" 2>/dev/null | grep -c .)
    sf compile >/dev/null 2>&1; sf_commit "verified $sf_w"
    if [[ $SF_SELF_RC -ne 0 && "$SF_SELF_N" == 0 && "$(grep -c "^ERROR verify.inadmissible" <<<"$SF_SELF")" -gt 0 && "$SF_IND_N" -gt 0 ]]; then
        sf_ok "independent verdicts on $sf_w" "the performer's own verdict refused, nothing written; the fixture verifier recorded"
    else
        sf_fail "independent verdicts on $sf_w" "self $SF_SELF_RC ($SF_SELF_N files): $(sf_lines "$SF_SELF") | independent $SF_IND_N: $(sf_lines "$SF_IND")"
    fi
    sf_resolution "$sf_w" claude "$SF_RESP/$sf_w.agent.resolution.response.toml"
    SF_OUT=$(sf resolve "$sf_w" --response "$SF_RESP/$sf_w.agent.resolution.response.toml" 2>&1); SF_RC=$?
    if [[ $SF_RC -ne 0 && ! -f "$SF_ROOT/docs/warrants/$sf_w/resolution.toml" ]] && grep -q '^ERROR resolution.agent' <<<"$SF_OUT"; then
        sf_ok "an agent cannot resolve $sf_w" "$(grep -m1 -oE '^ERROR +[a-z.-]+' <<<"$SF_OUT" | tr -s ' '); nothing written"
    else
        sf_fail "an agent cannot resolve $sf_w" "exit $SF_RC: $(sf_lines "$SF_OUT")"
    fi
    /usr/bin/rm -f "$SF_RESP/$sf_w.agent.resolution.response.toml"
    sf_resolution "$sf_w" "Plant Human" "$SF_RESP/$sf_w.resolution.response.toml"
    sf_sign "$SF_RESP/$sf_w.resolution.response.toml"
    SF_OUT=$(sf resolve "$sf_w" --response "$SF_RESP/$sf_w.resolution.response.toml" 2>&1); SF_RC=$?
    if [[ $SF_RC -eq 0 && -f "$SF_ROOT/docs/warrants/$sf_w/resolution.toml" ]]; then
        sf_ok "a person resolves $sf_w" "resolution.toml beside it"
    else
        sf_fail "a person resolves $sf_w" "exit $SF_RC: $(sf_lines "$SF_OUT")"
    fi
    sf compile >/dev/null 2>&1; sf_commit "resolved $sf_w"
done

# The resolved single file pins its delivered file like any Warrant.
SF_PINS=$(sf admin pins --resolved-only 2>&1)
if grep -q "$SF_A .*NOTE.md" <<<"$SF_PINS"; then
    sf_ok "its deliverable is pinned" "pins --resolved-only lists $SF_A D-001 NOTE.md"
else
    sf_fail "its deliverable is pinned" "$(head -3 <<<"$SF_PINS" | tr '\n' '|')"
fi
SF_FINAL=$(sf check 2>&1); SF_FINAL_RC=$?
[[ $SF_FINAL_RC -eq 0 ]] || sf_fail "the program checks after" "exit $SF_FINAL_RC: $(grep -E '^ERROR' <<<"$SF_FINAL" | head -3 | tr '\n' '|')"

/usr/bin/rm -rf "$SF_TMP"
unset SF_ROOT SF_TMP SF_RESP SF_NEW SF_A SF_DOC SF_STATE SF_D SF_DDIR sf_pair sf_w SF_CHECK SF_CHECK_RC SF_C1 SF_C2
unset SF_VERIFIER SF_SELF_N SF_IND SF_IND_N sf_dir sf_rest sf_file SF_PINS SF_OUT SF_RC SF_DRY SF_EV SF_SELF SF_SELF_RC SF_FINAL SF_FINAL_RC SF_SAS
