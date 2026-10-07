# shellcheck shell=bash
# OW-WAR-0112 — ownership (OW-ADR-0021) and the dry run.
#
# Every plant here runs on a scratch program or reads this corpus without
# mutating it. The dry-run plants come first because they are what an agent
# reaches for before asking a human to sign; the ownership plants follow in
# M1 phase B, once the drift rule itself lands in check.rs.

PLANT_ROOT=$(scratch_corpus DR)
# Sourced outside plant.sh, `scratch_corpus` is undefined and PLANT_ROOT is
# empty, and every `git -C "$PLANT_ROOT"` below would then act on the real
# repository — it did once, committing a working tree as "plant". Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
W1=DR-WAR-0001
# A scaffold ships the authority EXAMPLES only — nobody is eligible to sign,
# and a dry run rightly stops at `sign.who`. The plant needs an eligible
# human on the register, so the example becomes the register here. This is a
# throwaway program: the write-once rule for these files is about this
# repository's, not a scratch's.
cp "$PLANT_ROOT/docs/authority/roles.toml.example" "$PLANT_ROOT/docs/authority/roles.toml"
cp "$PLANT_ROOT/docs/authority/allowed_signers.example" "$PLANT_ROOT/docs/authority/allowed_signers"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register" >/dev/null 2>&1

# A fresh scaffold's adopt Warrant awaits authorization. A dry run must say
# it WOULD record, and must leave no trace: no response, no draft, no journal
# line, no attestation, and a tree byte-identical to before.
DR_BEFORE=$(git -C "$PLANT_ROOT" status --porcelain | sort)
# The example register names two humans, and `sign` rightly refuses to pick
# one; the plant picks, as an operator would.
DR_OUT=$("$WAR" --root "$PLANT_ROOT" sign "$W1" --dry-run --as your-name-here 2>&1)
DR_STATUS=$?
DR_AFTER=$(git -C "$PLANT_ROOT" status --porcelain | sort)
if [[ $DR_STATUS -eq 0 ]] && grep -q 'authorize.would-record' <<<"$DR_OUT" \
    && grep -q 'sign.would-record' <<<"$DR_OUT" \
    && [[ "$DR_BEFORE" == "$DR_AFTER" ]] \
    && [[ ! -d "$PLANT_ROOT/docs/authority/responses" || -z "$(ls -A "$PLANT_ROOT/docs/authority/responses" 2>/dev/null)" ]]; then
    printf 'ok    %-34s would record, wrote nothing\n' "dry run of an authorization"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; tree moved: %s\n' "dry run of an authorization" "$DR_STATUS" \
        "$([[ "$DR_BEFORE" == "$DR_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# The dry run must exercise the SAME refusals the real ingest does, and must
# never say an act was recorded. Over a whole queue: every act ends as
# `would-record` or `would-refuse` (or `needs-decision`, the sweep's own word
# for an act waiting on a flag), nothing ends as `<act>.recorded`, and a
# refusal is preceded by the ingest rule it came from. Not pinned to an alias
# of this corpus: the queue moves, and a plant pinned to a moment in it breaks
# the night the owner signs (lib.sh, `scratch_warrant`) — which is also why
# the queue is this scratch's, grown here to three acts of two kinds (a
# second authorization and a SAS acceptance beside DR-WAR-0001), and not this
# corpus's, which is empty whenever the owner has signed everything.
"$WAR" --root "$PLANT_ROOT" new "A second pending authorization" >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: war new in %s\n' "$PLANT_ROOT" >&2; exit 9; }
"$WAR" --root "$PLANT_ROOT" sas propose 0.1.0 >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: war sas propose in %s\n' "$PLANT_ROOT" >&2; exit 9; }
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a queue of three" >/dev/null 2>&1
DR_QUEUE=$("$WAR" --root "$PLANT_ROOT" sign --list 2>/dev/null | grep -cE '^  (DR-WAR-[0-9]{4}|SAS [0-9.]+) ')
[[ "$DR_QUEUE" -eq 3 ]] || { printf 'PLANT SETUP FAILED: wanted three pending acts, have %s\n' "$DR_QUEUE" >&2; exit 9; }
DR2_BEFORE=$(git -C "$PLANT_ROOT" status --porcelain | sort)
DR2_OUT=$("$WAR" --root "$PLANT_ROOT" sign --all --dry-run --as your-name-here 2>&1)
DR2_AFTER=$(git -C "$PLANT_ROOT" status --porcelain | sort)
DR2_JUDGED=$(grep -cE 'sign\.(would-record|would-refuse)' <<<"$DR2_OUT")
DR2_RECORDED=$(grep -cE '(authorize|resolution|correction)\.recorded|attest\.emitted' <<<"$DR2_OUT")
if [[ "$DR2_JUDGED" -gt 0 && "$DR2_RECORDED" -eq 0 ]] \
    && [[ "$DR2_BEFORE" == "$DR2_AFTER" ]] \
    && ! ls "$PLANT_ROOT"/docs/authority/responses/*.draft.toml >/dev/null 2>&1; then
    printf 'ok    %-34s %s act(s) judged, none recorded, nothing written\n' "dry run names the real refusal" "$DR2_JUDGED"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s judged %s, recorded %s, tree moved: %s\n' "dry run names the real refusal" "$DR2_JUDGED" "$DR2_RECORDED" \
        "$([[ "$DR2_BEFORE" == "$DR2_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# --all --dry-run over a whole queue: many acts, still nothing written, no
# draft left in a temp directory either. The temp directory is this run's own
# (TMPDIR), so a draft another `war` on the machine leaves in /tmp is not
# read as this run's, and one this run leaves is not hidden by it.
DR3_TMP=$(mktemp -d)
DR3_BEFORE=$(git -C "$PLANT_ROOT" status --porcelain | sort)
DR3_OUT=$(TMPDIR="$DR3_TMP" "$WAR" --root "$PLANT_ROOT" sign --all --dry-run --as your-name-here 2>&1)
DR3_AFTER=$(git -C "$PLANT_ROOT" status --porcelain | sort)
DR3_JUDGED=$(grep -cE 'sign\.(would-record|would-refuse)' <<<"$DR3_OUT")
if [[ "$DR3_JUDGED" -gt 0 && "$DR3_BEFORE" == "$DR3_AFTER" ]] && [[ -z "$(ls -A "$DR3_TMP" 2>/dev/null)" ]]; then
    printf 'ok    %-34s a whole queue judged, nothing written\n' "dry run of every pending act"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s tree moved or a temp draft survived: %s\n' "dry run of every pending act" "$(ls -A "$DR3_TMP" 2>/dev/null | tr '\n' ' ')"
    FAILED=$((FAILED + 1))
fi
command rm -rf "$DR3_TMP"

# This corpus's queue, whatever it holds today: the dry run is never silent
# about it — acts judged, or `sign.nothing-pending` said — and records and
# writes nothing either way.
DR4_TMP=$(mktemp -d)
DR4_BEFORE=$(git status --porcelain | sort)
DR4_OUT=$(TMPDIR="$DR4_TMP" "$WAR" sign --all --dry-run 2>&1)
DR4_AFTER=$(git status --porcelain | sort)
if { [[ $(grep -cE 'sign\.(would-record|would-refuse|needs-decision|who)' <<<"$DR4_OUT") -gt 0 ]] \
        || grep -q 'sign.nothing-pending' <<<"$DR4_OUT"; } \
    && ! grep -qE '(authorize|resolution|correction)\.recorded|attest\.emitted' <<<"$DR4_OUT" \
    && [[ "$DR4_BEFORE" == "$DR4_AFTER" ]] && [[ -z "$(ls -A "$DR4_TMP" 2>/dev/null)" ]]; then
    printf 'ok    %-34s %s\n' "this corpus's queue, dry run" "$(grep -oE 'sign\.nothing-pending' <<<"$DR4_OUT" || echo 'acts judged'), nothing written"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s silent, recorded, or wrote\n' "this corpus's queue, dry run"
    FAILED=$((FAILED + 1))
fi
command rm -rf "$DR4_TMP"

# A dry run holds no key: the code path must never reach ssh-keygen.
if ! grep -q 'ssh-keygen\|SSH_AUTH_SOCK' <<<"$DR_OUT$DR2_OUT"; then
    printf 'ok    %-34s no ssh in the transcript\n' "dry run reaches no key"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s ssh mentioned\n' "dry run reaches no key"
    FAILED=$((FAILED + 1))
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT

# ---------------------------------------------------------------------------
# Ownership (OW-ADR-0021), on THIS corpus. OW-WAR-0005 resolved with a pin on
# check.rs; OW-WAR-0112 later declared it and was authorized. Read-only
# plants grep one `war check`; mutating ones go through plant()/restore.
echo "== ownership (OW-ADR-0021) =="
OWN_OUT=$("$WAR" check 2>&1)

# A later owner makes the older pin historical: a PASS naming both Warrants,
# and no drift or correction error for that path against the old one. Which
# later Warrant owns check.rs moves as Warrants are authorized (0112, then
# 0114); the plant holds the rule, not today's owner.
if grep -qE 'deliverable.superseded-by .*OW-WAR-0005: D-001 pinned crates/openwarrant-cli/src/check.rs .*OW-WAR-0(1[0-9][0-9]|[2-9][0-9][0-9])/D-' <<<"$OWN_OUT" \
    && ! grep -qE '^ERROR .*OW-WAR-0005: D-001' <<<"$OWN_OUT"; then
    printf 'ok    %-34s OW-WAR-0005/D-001 historical under a later owner\n' "later owner makes the pin historical"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no superseded-by for OW-WAR-0005/D-001, or it still errors\n' "later owner makes the pin historical"
    FAILED=$((FAILED + 1))
fi

# `war pins` says the same thing the check does, from the same index.
if "$WAR" --json pins 2>/dev/null | python3 -c '
import sys, json
d = json.load(sys.stdin); r = d.get("result", d)
rows = {(p["warrant"], p["path"]): p for p in r["pins"]}
old = rows[("OW-WAR-0005", "crates/openwarrant-cli/src/check.rs")]
new = rows[("OW-WAR-0112", "crates/openwarrant-cli/src/check.rs")]
sys.exit(0 if old["historical"] and not new["historical"] else 1)
'; then
    printf 'ok    %-34s 0005 historical, 0112 current\n' "pins agree with the check"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s pins and check disagree on check.rs\n' "pins agree with the check"
    FAILED=$((FAILED + 1))
fi

# A resolved Warrant under an older SAS is history, not a re-pin debt.
if grep -q 'sas.pin-historical .*OW-WAR-0062' <<<"$OWN_OUT" && ! grep -q 'sas.pin-superseded .*OW-WAR-0062' <<<"$OWN_OUT"; then
    printf 'ok    %-34s resolved Warrant asked for nothing\n' "old SAS pin on a resolved Warrant"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s OW-WAR-0062 still warns sas.pin-superseded, or no pin-historical\n' "old SAS pin on a resolved Warrant"
    FAILED=$((FAILED + 1))
fi

# A correction against a historical pin is refused by name and writes nothing:
# there is nothing to correct, the newer owner answers for the bytes.
CH_BEFORE=$(git status --porcelain | sort)
CH_OUT=$("$WAR" correct OW-WAR-0005 D-001 2>&1)
CH_AFTER=$(git status --porcelain | sort)
if grep -q 'correction.historical' <<<"$CH_OUT" && [[ "$CH_BEFORE" == "$CH_AFTER" ]]; then
    printf 'ok    %-34s refused, nothing written\n' "correction of a historical pin"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; tree moved: %s\n' "correction of a historical pin" "$?" \
        "$([[ "$CH_BEFORE" == "$CH_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# A path added to deliverables.toml AFTER the signature is declared but not
# owned: a warning that says so, and no ownership of the new path.
plant_restore
printf '\n[[deliverable]]\nid = "D-999"\ntitle = "added after signing"\nkind = "file"\ntarget_ref = "docs/roadmap/PHASE1_EXIT.md"\nrequired = false\ncontent_addressed = false\nprovenance_required = false\n' \
    >> docs/warrants/OW-WAR-0112/deliverables.toml
assert_present 'D-999' docs/warrants/OW-WAR-0112/deliverables.toml
UA_OUT=$("$WAR" check 2>&1)
plant_restore
if grep -q 'deliverable.undeclared-at-authorization .*OW-WAR-0112: D-999' <<<"$UA_OUT" \
    && ! grep -q 'superseded-by .*OW-WAR-0061: D-001' <<<"$UA_OUT"; then
    printf 'ok    %-34s declared, not owned; OW-WAR-0061 keeps its pin\n' "path added after signing"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no undeclared-at-authorization for D-999, or 0061 went historical\n' "path added after signing"
    FAILED=$((FAILED + 1))
fi

# A signed-for deliverable deleted from the declaration is an error: a claim
# on a path is not withdrawn by removing the line.
plant "a signed deliverable removed from the declaration" "deliverable.declared-then-removed" \
    "OW-WAR-0112: the authorization signed for D-001" 2 \
    "python3 -c \"
import pathlib, re
p = pathlib.Path('docs/warrants/OW-WAR-0112/deliverables.toml')
t = p.read_text()
t2 = re.sub(r'\[\[deliverable\]\]\nid = \\\"D-001\\\"\n(?:.*\n)*?(?=\[\[deliverable\]\])', '', t, count=1)
assert t2 != t
p.write_text(t2)
\"; ! grep -q 'id = \"D-001\"' docs/warrants/OW-WAR-0112/deliverables.toml"

# The `owned` set lives under the authorization's attestation: editing it is
# record tampering, and `war attest --verify` says so before any ownership is
# computed from it.
plant_restore
sed -i 's#docs/adr/atoms/OW-ADR-0021-pin-ownership.md#docs/adr/atoms/OW-ADR-0021-x.md#' docs/warrants/OW-WAR-0112/authorization.toml
assert_present 'OW-ADR-0021-x.md' docs/warrants/OW-WAR-0112/authorization.toml
OE_OUT=$("$WAR" attest OW-WAR-0112 --verify 2>&1)
OE_STATUS=$?
plant_restore
if [[ $OE_STATUS -eq 2 ]] && grep -q 'attest.subject-drift .*OW-WAR-0112/authorization.toml' <<<"$OE_OUT"; then
    printf 'ok    %-34s attest.subject-drift (exit 2)\n' "owned set edited after signing"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, subject-drift on authorization.toml not reported\n' "owned set edited after signing" "$OE_STATUS"
    FAILED=$((FAILED + 1))
fi

# ---------------------------------------------------------------------------
# Ownership, signed, on a scratch program (OW-WAR-0112 OBL-001, OBL-002,
# OBL-003). Two Warrants declare the same path, ADOPTED.md: the scaffold's
# adoption Warrant (OS-WAR-0001) and one from `war new` (OS-WAR-0002). A plant
# human's throwaway key signs every response — `ssh-keygen -Y sign -f`, never
# an agent, SSH_AUTH_SOCK unset — because ownership is granted only by a
# verified signature: an unsigned record owns nothing, so no refusal below
# would have an owner to refuse against. Nothing here is this corpus.
echo "== ownership, signed on a scratch program (OW-WAR-0112) =="
OS_ROOT=$(scratch_corpus OS)
[[ -d "${OS_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
OS_TMP=$(mktemp -d)
OS_A=OS-WAR-0001
OS_B=OS-WAR-0002
OS_ADIR="$OS_ROOT/docs/warrants/$OS_A"
OS_BDIR="$OS_ROOT/docs/warrants/$OS_B"
# Signatures are found where `authority_check` looks: docs/authority/responses/.
OS_RESP="$OS_ROOT/docs/authority/responses"
# The scaffold's gate is `war check --generated` by name: the war built here
# must be the one it finds.
os()        { PATH="$REPO_ROOT/target/debug:$PATH" "$REPO_ROOT/${WAR#./}" --root "$OS_ROOT" "$@"; }
os_commit() {
    git -C "$OS_ROOT" add -A >/dev/null 2>&1
    git -C "$OS_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgsign=false \
        commit -qm "$1" >/dev/null 2>&1
}
os_tree()   { git -C "$OS_ROOT" status --porcelain | sort; }
os_sign()   { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID ssh-keygen -q -Y sign -f "$OS_TMP/id_plant" -n oh.war/response "$1" >/dev/null 2>&1; }
os_setup_failed() { printf 'PLANT SETUP FAILED: %s in %s\n' "$1" "$OS_ROOT" >&2; exit 9; }
os_ok()     { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
os_fail()   { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
os_lines()  { grep -E '^(PASS|ERROR|UNKNOWN|WARN)' <<<"$1" | head -3 | tr '\n' '|'; }
# An authorization response: alias, effective time, set digest.
os_auth_response() {
    local digest
    digest=$(os authorize "$1" 2>/dev/null | grep '^contract_digest' | cut -d'"' -f2)
    [[ -n "$digest" ]] || os_setup_failed "no contract digest for $1"
    printf 'schema = "oh.war/authorization-response/v1"\nwarrant = "%s"\ncontract_digest = "%s"\nauthorizer = "Plant Human"\nacting_role = "authorizer"\nmeaning = "plant fixture"\neffective_time = "%s"\nindependence = "separate_role"\ndeliverable_set_digest = "%s"\n' \
        "$1" "$digest" "$2" "$3"
}
os_set_digest() { os authorize "$1" 2>/dev/null | grep '^deliverable_set_digest' | cut -d'"' -f2; }
# A hook event: an Edit of <path> in the scratch program.
os_hook() {
    printf '{"session_id":"plant","hook_event_name":"PreToolUse","tool_name":"Edit","cwd":"%s","tool_input":{"file_path":"%s","old_string":"a","new_string":"b"}}' \
        "$OS_ROOT" "$1" \
        | PATH="$REPO_ROOT/target/debug:$PATH" bash "$REPO_ROOT/.claude/hooks/guard-pins.sh" 2>/dev/null
}

env -u SSH_AUTH_SOCK -u SSH_AGENT_PID ssh-keygen -q -t ed25519 -N "" -C plant -f "$OS_TMP/id_plant" \
    || os_setup_failed "ssh-keygen"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$OS_TMP/id_plant.pub")" \
    > "$OS_ROOT/docs/authority/allowed_signers"
cat > "$OS_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Human"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/98-ownership.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the ownership plants run; its key is generated and discarded here."
ssh_principal = "plant"

[[assignment]]
actor = "claude"
actor_kind = "agent"
roles = ["performer"]
assigned_by = "conformance/plants.d/98-ownership.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "The performer."
ROLES
mkdir -p "$OS_RESP"
os sas propose 0.1.0 >/dev/null 2>&1 || os_setup_failed "war sas propose"
OS_SAS=$(grep '^sha256' "$OS_ROOT/docs/sas/revisions/0.1.0.toml" 2>/dev/null | cut -d'"' -f2)
printf 'schema = "oh.war/sas-acceptance-response/v1"\nversion = "0.1.0"\nsha256 = "%s"\naccepted_by = "Plant Human"\nacting_role = "owner"\nmeaning = "plant fixture"\neffective_time = "2026-09-02T00:00:00Z"\n' \
    "$OS_SAS" > "$OS_RESP/SAS-0.1.0.response.toml"
os_sign "$OS_RESP/SAS-0.1.0.response.toml"
os sas accept 0.1.0 --response "$OS_RESP/SAS-0.1.0.response.toml" >/dev/null 2>&1 || os_setup_failed "war sas accept"
python3 - "$OS_ROOT/openwarrant.toml" "$REPO_ROOT/conformance/fixtures/verifier/establishes-all.sh" <<'PY' || os_setup_failed "the verifier fixture"
import sys
path, verifier = sys.argv[1], sys.argv[2]
text = open(path).read()
if "verifier_argv = []" not in text:
    sys.exit(1)
open(path, "w").write(text.replace("verifier_argv = []", f'verifier_argv = ["{verifier}"]', 1))
PY
printf 'adopted\n' > "$OS_ROOT/ADOPTED.md"
cat > "$OS_ADIR/deliverables.toml" <<TOML
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "The adoption note"
kind = "file"
target_ref = "ADOPTED.md"
required = true
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001", "OBL-002"]

[deliverable.provenance]
producer = "conformance/plants.d/98-ownership.sh"
producing_attempt = "$OS_A/attempt-1"
contract_digest = "unrecorded"
input_digests = []
tool_or_runtime_identity = "printf"
creation_method = "generated"
content_digest = "sha256:$(sha256sum < "$OS_ROOT/ADOPTED.md" | cut -d' ' -f1)"
media_type = "text/markdown"
classification = "internal"
retention = "repository-lifetime"
source_holder = "git"
TOML
printf 'schema = "oh.war/rationale/v1"\n' > "$OS_ADIR/rationale.toml"
os new "The second owner of ADOPTED.md" >/dev/null 2>&1 || os_setup_failed "war new"
[[ -d "$OS_BDIR" ]] || os_setup_failed "no $OS_B"
cat > "$OS_BDIR/deliverables.toml" <<'TOML'
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "The adoption note, taken over"
kind = "file"
target_ref = "ADOPTED.md"
required = false
content_addressed = false
provenance_required = false
TOML
os compile >/dev/null 2>&1
os_commit "two Warrants declare ADOPTED.md" || os_setup_failed "baseline commit"

# `war sign --show`: "Grants ownership of:" and then every declared path.
# Refusal side: it is a rendering, so the tree does not move.
OS_BEFORE=$(os_tree)
OS_OUT=$(os sign "$OS_A" --show --as "Plant Human" 2>&1)
if grep -q '│ Grants ownership of: 1 path(s)' <<<"$OS_OUT" \
    && [[ $(sed -n '/Grants ownership of:/,$p' <<<"$OS_OUT" | grep -cE '^│   D-[0-9]{3}  ') -eq 1 ]] \
    && grep -q '│   D-001  ADOPTED.md' <<<"$OS_OUT" && [[ "$OS_BEFORE" == "$(os_tree)" ]]; then
    os_ok "sign --show lists what it grants" "Grants ownership of: D-001 ADOPTED.md; nothing written"
else
    os_fail "sign --show lists what it grants" "$(grep -i -A2 'ownership' <<<"$OS_OUT" | tr '\n' '|')"
fi

# A response signing a set the manifest does not hold: refused by name, and
# nothing written — no authorization.toml, no journal line.
os_auth_response "$OS_A" 2026-09-20T00:00:00Z "sha256:$(printf '0%.0s' {1..64})" > "$OS_RESP/$OS_A.stale.response.toml"
os_sign "$OS_RESP/$OS_A.stale.response.toml"
OS_BEFORE=$(os_tree)
OS_OUT=$(os authorize "$OS_A" --response "$OS_RESP/$OS_A.stale.response.toml" 2>&1); OS_STATUS=$?
if [[ $OS_STATUS -eq 2 ]] && grep -q '^ERROR authorize.stale-deliverables' <<<"$OS_OUT" \
    && [[ ! -e "$OS_ADIR/authorization.toml" ]] && [[ "$OS_BEFORE" == "$(os_tree)" ]]; then
    os_ok "a response over a moved set" "authorize.stale-deliverables (exit 2), nothing written"
else
    os_fail "a response over a moved set" "exit $OS_STATUS: $(os_lines "$OS_OUT")"
fi
command rm -f "$OS_RESP/$OS_A.stale.response.toml" "$OS_RESP/$OS_A.stale.response.toml.sig"

# The paired control: the same response over the set the manifest holds is
# recorded, and OS-WAR-0001 now owns ADOPTED.md from 2026-09-20.
os_auth_response "$OS_A" 2026-09-20T00:00:00Z "$(os_set_digest "$OS_A")" > "$OS_RESP/$OS_A.response.toml"
os_sign "$OS_RESP/$OS_A.response.toml"
OS_OUT=$(os authorize "$OS_A" --response "$OS_RESP/$OS_A.response.toml" 2>&1); OS_STATUS=$?
if [[ $OS_STATUS -eq 0 ]] && grep -q '^PASS authorize.recorded' <<<"$OS_OUT" \
    && grep -q 'target_ref = "ADOPTED.md"' "$OS_ADIR/authorization.toml" 2>/dev/null; then
    os_ok "the signed set is recorded" "authorize.recorded; owned: ADOPTED.md"
else
    os_fail "the signed set is recorded" "exit $OS_STATUS: $(os_lines "$OS_OUT")"
fi
os_commit "OS-WAR-0001 authorized"

# A second Warrant dated BEFORE the owner it would displace: refused by name,
# nothing written.
os_auth_response "$OS_B" 2026-09-10T00:00:00Z "$(os_set_digest "$OS_B")" > "$OS_RESP/$OS_B.early.response.toml"
os_sign "$OS_RESP/$OS_B.early.response.toml"
OS_BEFORE=$(os_tree)
OS_OUT=$(os authorize "$OS_B" --response "$OS_RESP/$OS_B.early.response.toml" 2>&1); OS_STATUS=$?
if [[ $OS_STATUS -eq 2 ]] && line_has -E '^ERROR authorize.time-before-owner' -F "when $OS_A was authorized for ADOPTED.md" <<<"$OS_OUT" \
    && [[ ! -e "$OS_BDIR/authorization.toml" ]] && [[ "$OS_BEFORE" == "$(os_tree)" ]]; then
    os_ok "an authorization dated before owner" "authorize.time-before-owner naming $OS_A (exit 2), nothing written"
else
    os_fail "an authorization dated before owner" "exit $OS_STATUS: $(os_lines "$OS_OUT")"
fi
command rm -f "$OS_RESP/$OS_B.early.response.toml" "$OS_RESP/$OS_B.early.response.toml.sig"

# OS-WAR-0001 resolved: evidence over the committed tree, the fixture
# verifier, and a signed resolution. The record carries a [locator] whose
# commit is the tree it was resolved at.
# Authorization changes the generated status. Compile before the check gate
# runs so the first receipt is a passing observation, then review it once.
os compile >/dev/null 2>&1; os_commit "ready for evidence"
if ! OS_EVIDENCE=$(os evidence record "$OS_A" 2>&1); then
    printf '%s\n' "$OS_EVIDENCE" >&2
    os_setup_failed "the initial evidence gate"
fi
os compile >/dev/null 2>&1; os_commit "evidence"
os verify "$OS_A" --performer claude --run >/dev/null 2>&1; os compile >/dev/null 2>&1; os_commit "verified"
OS_HEAD=$(git -C "$OS_ROOT" rev-parse HEAD)
OS_DIGEST=$(os authorize "$OS_A" 2>/dev/null | grep '^contract_digest' | cut -d'"' -f2)
printf 'schema = "oh.war/resolution-response/v1"\nwarrant = "%s"\ncontract_digest = "%s"\nresolved_by = "Plant Human"\nacting_role = "resolver"\ncommon_outcome = "satisfied"\nprofile_outcome = "delivered"\nmeaning = "plant fixture"\neffective_time = "2026-09-22T00:00:00Z"\n' \
    "$OS_A" "$OS_DIGEST" > "$OS_RESP/$OS_A.resolution.response.toml"
os_sign "$OS_RESP/$OS_A.resolution.response.toml"
OS_OUT=$(os resolve "$OS_A" --response "$OS_RESP/$OS_A.resolution.response.toml" 2>&1); OS_STATUS=$?
OS_LOC=$(python3 -c '
import sys, tomllib
r = tomllib.load(open(sys.argv[1], "rb"))
print(r.get("locator", {}).get("commit_sha", ""))' "$OS_ADIR/resolution.toml" 2>/dev/null)
if [[ $OS_STATUS -eq 0 ]] && [[ "$OS_LOC" =~ ^[0-9a-f]{40}$ ]] && [[ "$OS_LOC" == "$OS_HEAD" ]]; then
    os_ok "a resolution carries its locator" "[locator] commit_sha = ${OS_LOC:0:12}… (40 lowercase hex, the resolved HEAD)"
else
    os_fail "a resolution carries its locator" "exit $OS_STATUS, commit_sha '$OS_LOC', HEAD $OS_HEAD: $(os_lines "$OS_OUT")"
fi
os compile >/dev/null 2>&1; os_commit "OS-WAR-0001 resolved"

# While OS-WAR-0001 is the only owner its pin is current: the hook denies the
# edit and names the correction act.
OS_HOOK=$(os_hook "$OS_ROOT/ADOPTED.md")
if grep -q '"permissionDecision":"deny"' <<<"$OS_HOOK" && grep -q "war correct $OS_A D-001" <<<"$OS_HOOK"; then
    os_ok "the hook denies a current pin" "deny, names war correct $OS_A D-001"
else
    os_fail "the hook denies a current pin" "hook said: $(cut -c1-200 <<<"$OS_HOOK")"
fi

# OBL-003 over the same program: `war check --json` on a planted drift and on
# a planted stale projection. The drift is the resolved pin moved: a HUMAN
# remedy naming the correction and the signature it ends in. The stale
# projection is an atom edited without `war compile`: an AUTO `war compile`.
printf 'moved\n' >> "$OS_ROOT/ADOPTED.md"
printf '\nA planted line.\n' >> "$OS_BDIR/atoms/10-intent.md"
OS_JSON=$(os check --generated --json 2>/dev/null)
OS_HUMAN=$(os check --generated 2>&1)
corpus_reset "$OS_ROOT"
if python3 -c '
import sys, json
alias = sys.argv[1]
d = json.load(sys.stdin)
drift = [x for x in d["diagnostics"] if x["rule"] == "deliverable.digest-drift"]
assert drift, "no drift"
r = drift[0]["remedy"]
assert r["kind"] == "human", r
assert r["argv"] == ["war", "correct", alias, "D-001"], r
assert f"war sign {alias}/D-001 --ssh-sign" in r["purpose"], r
stale = [x for x in d["diagnostics"] if x["rule"] == "generated.drift" and x["severity"] != "pass"]
assert stale, "no stale projection"
for x in stale:
    assert x["remedy"]["kind"] == "auto" and x["remedy"]["argv"] == ["war", "compile"], x
assert d["exit_code"] == 2 and d["verdict"] == "not_ready", d["verdict"]
' "$OS_A" <<<"$OS_JSON" 2>"$OS_TMP/py.err" \
    && [[ $(grep -c '^REMEDIES:' <<<"$OS_HUMAN") -eq 1 ]] \
    && grep -qE '^  human +war sign correct OS-WAR-0001 D-001 ' <<<"$OS_HUMAN" \
    && grep -qE '^  auto +war admin compile ' <<<"$OS_HUMAN"; then
    os_ok "check --json carries each remedy" "drift: human war correct (signs with war sign $OS_A/D-001); stale: auto war compile; REMEDIES: block"
else
    os_fail "check --json carries each remedy" "$(tail -1 "$OS_TMP/py.err") / $(grep -A3 '^REMEDIES:' <<<"$OS_HUMAN" | tr '\n' '|')"
fi

# OS-WAR-0002 at a time AFTER the owner: recorded, and it takes the path.
os_auth_response "$OS_B" 2026-09-25T00:00:00Z "$(os_set_digest "$OS_B")" > "$OS_RESP/$OS_B.response.toml"
os_sign "$OS_RESP/$OS_B.response.toml"
OS_OUT=$(os authorize "$OS_B" --response "$OS_RESP/$OS_B.response.toml" 2>&1); OS_STATUS=$?
if [[ $OS_STATUS -eq 0 ]] && grep -q '^PASS authorize.recorded' <<<"$OS_OUT"; then
    os_ok "a later owner is recorded" "$OS_B authorized after $OS_A"
else
    os_fail "a later owner is recorded" "exit $OS_STATUS: $(os_lines "$OS_OUT")"
fi
os_commit "OS-WAR-0002 authorized"

# The resolved pin is now historical: the same moved byte that was drift
# above is passed by name as superseded, and the hook lets through the edit
# it denied above, on the same path.
printf 'moved\n' >> "$OS_ROOT/ADOPTED.md"
OS_OUT=$(os check 2>&1)
OS_HOOK=$(os_hook "$OS_ROOT/ADOPTED.md")
corpus_reset "$OS_ROOT"
if line_has -E '^PASS deliverable.superseded-by' -F "$OS_A: D-001" <<<"$OS_OUT" \
    && ! grep -qE "^ERROR +deliverable\..*$OS_A: D-001" <<<"$OS_OUT" && [[ -z "$OS_HOOK" ]]; then
    os_ok "the hook permits a historical pin" "superseded-by $OS_B; the edit it denied is let through"
else
    os_fail "the hook permits a historical pin" "hook: $(cut -c1-120 <<<"$OS_HOOK"); $(grep -m2 "$OS_A: D-001" <<<"$OS_OUT" | tr '\n' '|')"
fi

# `war pins --history`: both owners, oldest first — the order they took it.
OS_OUT=$(os pins --history ADOPTED.md 2>&1)
OS_FIRST=$(grep -nE "^  2026-09-20T00:00:00Z +$OS_A/D-001 +historical → $OS_B" <<<"$OS_OUT" | cut -d: -f1)
OS_SECOND=$(grep -nE "^  2026-09-25T00:00:00Z +$OS_B/D-001 +current" <<<"$OS_OUT" | cut -d: -f1)
if [[ -n "$OS_FIRST" && -n "$OS_SECOND" ]] && [[ "$OS_FIRST" -lt "$OS_SECOND" ]] \
    && [[ $(grep -cE '^  [0-9—]' <<<"$OS_OUT") -eq 2 ]]; then
    os_ok "pins --history, oldest first" "$OS_A (historical) then $OS_B (current)"
else
    os_fail "pins --history, oldest first" "$(tr '\n' '|' <<<"$OS_OUT")"
fi

corpus_gone "$OS_ROOT"
command rm -rf "$OS_TMP"
