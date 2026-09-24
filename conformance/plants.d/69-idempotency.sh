# shellcheck shell=bash
# OW-WAR-0130 — idempotent acts, a batch killed mid-record, and version
# negotiation between war and its records.
#
# Scratch programs only, a key generated here, and a throwaway ssh-agent that
# holds that key and nothing else. Nothing here reads or signs with the
# owner's key or touches this repository's corpus.
#
# Every claim is paired with the control seen refusing, and every refusal is
# matched by the rule that fired, not by a non-zero exit. The crash is the
# debug build's hook OPENWARRANT_TEST_BATCH_KILL_AFTER=<n>: the process sends
# itself SIGKILL after its n-th record, so nothing in it gets to clean up.
#
# OBL-002's negative control ("before this change the same plant fails on
# `war verify --response`") needs a binary built before the change. Set
# IP_WAR_BEFORE to one and the plant runs the same retry with it; unset, that
# one line says it was not run rather than passing.

echo "== idempotency, batch recovery, version negotiation (OW-WAR-0130) =="
IP_TMP=$(mktemp -d)
ip_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ip_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ip_errors() { grep -E '^(ERROR|WARN|UNKNOWN|error)' <<<"$1" | head -3 | tr '\n' '|'; }
# Every file of a directory by content: the "nothing was written" measure.
ip_tree() { (cd "$1" && find . -type f -print0 | sort -z | xargs -0 sha256sum) | sha256sum | cut -d' ' -f1; }
ip_sha() { if [[ -e "$1" ]]; then echo "sha256:$(sha256sum < "$1" | cut -d' ' -f1)"; else echo absent; fi; }

# A program with three Warrants, the plant key registered for every role.
ip_corpus() { # namespace → root on stdout
    local root
    root=$(scratch_corpus "$1")
    [[ -d "${root:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
    printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$IP_PUB" > "$root/docs/authority/allowed_signers"
    cat > "$root/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/69-idempotency.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the idempotency plants run."
ssh_principal = "plant"
ROLES
    "$WAR" --root "$root" new "A second Warrant for the batch" >/dev/null 2>&1
    "$WAR" --root "$root" new "A third Warrant for the batch" >/dev/null 2>&1
    "$WAR" --root "$root" compile >/dev/null 2>&1
    git -C "$root" add -A >/dev/null 2>&1
    git -C "$root" -c user.email=plant@invalid -c user.name=plant commit -qm "register, three Warrants" >/dev/null 2>&1
    printf '%s' "$root"
}

ssh-keygen -q -t ed25519 -N "" -C plant -f "$IP_TMP/id_plant"
IP_PUB=$(cut -d' ' -f1,2 "$IP_TMP/id_plant.pub")
IP_OLD_SOCK=${SSH_AUTH_SOCK:-}
eval "$(ssh-agent -s > "$IP_TMP/agent.env"; cat "$IP_TMP/agent.env")" >/dev/null
ssh-add -q "$IP_TMP/id_plant" 2>/dev/null
# The agent must hold this key and nothing else.
IP_KEYS=$(ssh-add -l 2>/dev/null)
IP_FP=$(ssh-keygen -lf "$IP_TMP/id_plant.pub" | awk '{print $2}')
if [[ $(grep -c . <<<"$IP_KEYS") -ne 1 ]] || ! grep -qF -- "$IP_FP" <<<"$IP_KEYS"; then
    printf 'PLANT SETUP FAILED: the throwaway agent holds more than the plant key:\n%s\n' "$IP_KEYS" >&2
    ssh-agent -k >/dev/null 2>&1
    exit 9
fi

# ── OBL-001: a batch killed mid-record is found and undone ──────────────────
PLANT_ROOT=$(ip_corpus IB)
IB=$PLANT_ROOT
ib() { "$WAR" --root "$IB" "$@"; }
ib_batch() { ib sign --batch --ssh-sign --as "Plant Signer" </dev/null; }
# docs/ by content, the batches directory aside (a refused batch is kept there).
ib_docs() { (cd "$IB" && find docs -path docs/authority/batches -prune -o -type f -print0 | sort -z | xargs -0 sha256sum) | sha256sum | cut -d' ' -f1; }
IB_PENDING=$(ib sign --list 2>&1 | grep -c 'authorize')
[[ "$IB_PENDING" -eq 3 ]] || { printf 'PLANT SETUP FAILED: wanted three pending authorizations, have %s\n' "$IB_PENDING" >&2; exit 9; }

# The paired control first: a program with no marker reports no interruption.
IB_OUT=$(ib check 2>&1)
if grep -q 'batch.interrupted' <<<"$IB_OUT"; then
    ip_fail "no marker, no batch.interrupted" "$(grep 'batch.interrupted' <<<"$IB_OUT" | head -1)"
else
    ip_ok "no marker, no batch.interrupted" "batch.interrupted silent"
fi

# Killed with SIGKILL after its second of three records.
IB_BEFORE=$(ib_docs)
IB_OUT=$(OPENWARRANT_TEST_BATCH_KILL_AFTER=2 ib_batch 2>&1); IB_STATUS=$?
IB_MARKER=$(ls "$IB"/docs/authority/batches/*.recording 2>/dev/null | head -1)
IB_ID=$(basename "${IB_MARKER:-none}" .recording)
IB_AUTH=$(ls "$IB"/docs/warrants/*/authorization.toml 2>/dev/null | wc -l)
if [[ $IB_STATUS -eq 137 ]] && [[ -n "$IB_MARKER" ]] && [[ "$IB_AUTH" -eq 2 ]]; then
    ip_ok "SIGKILL after 2 of 3 records" "killed (137), marker $IB_ID, 2 of 3 recorded: a hole"
else
    ip_fail "SIGKILL after 2 of 3 records" "exit $IB_STATUS, marker [${IB_MARKER:-none}], $IB_AUTH authorization(s): $(ip_errors "$IB_OUT")"
fi
cp "${IB_MARKER:-/dev/null}" "$IP_TMP/marker.json" 2>/dev/null
IB_LISTED=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["paths"]))' "$IP_TMP/marker.json" 2>/dev/null || echo 0)

# war check names it, as an ERROR, with the batch and its paths.
IB_OUT=$(ib check 2>&1); IB_STATUS=$?
if [[ $IB_STATUS -ne 0 ]] && grep -E '^ERROR +batch\.interrupted' <<<"$IB_OUT" | grep -q -- "$IB_ID" \
    && grep 'batch.interrupted' <<<"$IB_OUT" | grep -q 'authorization.toml'; then
    ip_ok "war check names the interrupted batch" "ERROR batch.interrupted $IB_ID, its paths listed"
else
    ip_fail "war check names the interrupted batch" "exit $IB_STATUS: $(grep -E 'batch\.' <<<"$IB_OUT" | head -2 | tr '\n' '|')"
fi
# And a new batch, even a dry run, refuses the same.
IB_OUT=$(ib sign --batch --as "Plant Signer" --dry-run </dev/null 2>&1); IB_STATUS=$?
if [[ $IB_STATUS -eq 2 ]] && grep -E '^ERROR +batch\.interrupted' <<<"$IB_OUT" | grep -q -- "$IB_ID" \
    && ! grep -q 'batch.would-record' <<<"$IB_OUT"; then
    ip_ok "a new batch refuses while interrupted" "batch.interrupted (dry run, exit 2)"
else
    ip_fail "a new batch refuses while interrupted" "exit $IB_STATUS: $(ip_errors "$IB_OUT")"
fi

# Recovery of a batch that is not interrupted is refused by name.
IB_OUT=$(ib sign --batch recover:B-NOT-A-BATCH 2>&1); IB_STATUS=$?
if [[ $IB_STATUS -eq 2 ]] && grep -q 'batch.nothing-to-recover' <<<"$IB_OUT" && [[ -f "$IB_MARKER" ]]; then
    ip_ok "recovering an unknown batch refuses" "batch.nothing-to-recover; the marker stays"
else
    ip_fail "recovering an unknown batch refuses" "exit $IB_STATUS: $(ip_errors "$IB_OUT")"
fi

# --recover: every path the marker lists is its prior bytes again, or absent.
IB_OUT=$(ib sign --batch "recover:$IB_ID" 2>&1); IB_STATUS=$?
IB_MOVED=$(python3 - "$IP_TMP/marker.json" "$IB" <<'PY'
import hashlib, json, os, sys
m = json.load(open(sys.argv[1])); root = sys.argv[2]
for l in m["paths"]:
    p = os.path.join(root, l["path"])
    now = "sha256:" + hashlib.sha256(open(p, "rb").read()).hexdigest() if os.path.isfile(p) else "absent"
    if now != l["prior"]:
        print(l["path"], l["prior"][:15], now[:15])
PY
)
if [[ $IB_STATUS -eq 0 ]] && grep -q 'batch.recovered' <<<"$IB_OUT" && [[ "$IB_LISTED" -gt 0 ]] && [[ -z "$IB_MOVED" ]] \
    && [[ -f "$IB/docs/authority/batches/$IB_ID.refused.json" ]] && [[ ! -e "$IB_MARKER" ]] \
    && [[ ! -e "$IB/docs/authority/batches/$IB_ID.recording.d" ]] && [[ "$(ib_docs)" == "$IB_BEFORE" ]]; then
    ip_ok "recover restores every listed path" "$IB_LISTED path(s) at their prior sha256 or absent; .refused.json kept"
else
    ip_fail "recover restores every listed path" "exit $IB_STATUS, $IB_LISTED listed, moved [$IB_MOVED]: $(ip_errors "$IB_OUT")"
fi
IB_OUT=$(ib check 2>&1)
if grep -q 'batch.interrupted' <<<"$IB_OUT"; then
    ip_fail "after recovery, no batch.interrupted" "$(grep 'batch.interrupted' <<<"$IB_OUT" | head -1)"
else
    ip_ok "after recovery, no batch.interrupted" "batch.interrupted silent"
fi
command rm -f "$IB"/docs/authority/batches/*.refused.json*

# The in-process failures leave no marker either, and restore every path:
# a planted refusal after one act, and a target no dry run sees is
# unwritable (the third Warrant's journal, read-only).
IB_OUT=$(OPENWARRANT_TEST_BATCH_FAIL_AFTER=1 ib_batch 2>&1); IB_STATUS=$?
if [[ $IB_STATUS -eq 2 ]] && grep -q 'batch.incomplete' <<<"$IB_OUT" && [[ "$(ib_docs)" == "$IB_BEFORE" ]] \
    && [[ -z "$(ls "$IB"/docs/authority/batches/*.recording* 2>/dev/null)" ]]; then
    ip_ok "a planted refusal leaves no marker" "batch.incomplete, docs/ byte-identical"
else
    ip_fail "a planted refusal leaves no marker" "exit $IB_STATUS: $(ip_errors "$IB_OUT")"
fi
command rm -f "$IB"/docs/authority/batches/*.refused.json*
IB_J3=$(ls -d "$IB"/docs/warrants/IB-WAR-0003 2>/dev/null)/journal.jsonl
chmod a-w "$IB_J3"
IB_BEFORE_RO=$(ib_docs)
IB_OUT=$(ib_batch 2>&1); IB_STATUS=$?
chmod u+w "$IB_J3"
if [[ $IB_STATUS -eq 2 ]] && grep -q 'batch.incomplete' <<<"$IB_OUT" && [[ "$(ib_docs)" == "$IB_BEFORE_RO" ]] \
    && [[ -z "$(ls "$IB"/docs/warrants/*/authorization.toml 2>/dev/null)" ]] \
    && [[ -z "$(ls "$IB"/docs/authority/batches/*.recording* 2>/dev/null)" ]]; then
    ip_ok "an unwritable target leaves no marker" "batch.incomplete, docs/ byte-identical"
else
    ip_fail "an unwritable target leaves no marker" "exit $IB_STATUS: $(ip_errors "$IB_OUT")"
fi
command rm -f "$IB"/docs/authority/batches/*.refused.json*

# The same three acts, not killed: n of n, and no marker left behind.
IB_OUT=$(ib_batch 2>&1); IB_STATUS=$?
if [[ $IB_STATUS -eq 0 ]] && grep -q 'batch.recorded.*3 of 3' <<<"$IB_OUT" \
    && [[ $(ls "$IB"/docs/warrants/*/authorization.toml 2>/dev/null | wc -l) -eq 3 ]] \
    && [[ -z "$(ls "$IB"/docs/authority/batches/*.recording* 2>/dev/null)" ]]; then
    ip_ok "an unkilled batch leaves no marker" "3 of 3 recorded, no .recording, no .recording.d"
else
    ip_fail "an unkilled batch leaves no marker" "exit $IB_STATUS: $(ip_errors "$IB_OUT") $(ls "$IB"/docs/authority/batches)"
fi
corpus_gone "$IB"
unset PLANT_ROOT

# ── OBL-002: an equivalent retry replays; a conflicting one is refused ──────
PLANT_ROOT=$(ip_corpus IA)
IA=$PLANT_ROOT
IA_A="IA-WAR-0001"
IA_DIR="$IA/docs/warrants/$IA_A"
IA_J="$IA_DIR/journal.jsonl"
ia() { "$WAR" --root "$IA" "$@"; }
# A stage names what runs it, or nothing can be dispatched to answer.
sed -i 's|^    executor_kind: "agent"$|    executor_kind: "agent"\n    executor_ref: "agent://plant"|' "$IA_DIR/atoms/45-milestones.yaml"
assert_present 'executor_ref: "agent://plant"' "$IA_DIR/atoms/45-milestones.yaml"
ia compile >/dev/null 2>&1
git -C "$IA" add -A >/dev/null 2>&1
git -C "$IA" -c user.email=plant@invalid -c user.name=plant commit -qm "stages name their executor" >/dev/null 2>&1
# `war evidence record` runs the program's gate, whose argv is `war check`.
mkdir -p "$IP_TMP/bin"
ln -sf "$(realpath "$WAR")" "$IP_TMP/bin/war"

# ia_retry <name> <replay-rule> <war args…>: the second run of the same act
# exits 0, says it replayed, and leaves the Warrant directory byte-identical.
ia_retry() {
    local name=$1 rule=$2 before out status
    shift 2
    before=$(ip_tree "$IA_DIR")
    out=$(PATH="$IP_TMP/bin:$PATH" ia "$@" </dev/null 2>&1); status=$?
    if [[ $status -eq 0 ]] && grep -qE "PASS +$rule" <<<"$out" && [[ "$(ip_tree "$IA_DIR")" == "$before" ]]; then
        ip_ok "$name" "$rule, exit 0, $IA_A byte-identical"
    else
        ip_fail "$name" "exit $status, tree $([[ "$(ip_tree "$IA_DIR")" == "$before" ]] && echo same || echo moved): $(ip_errors "$out")"
    fi
}
# ia_conflict <name> <event type> <war args…>: the journal's line for the
# event is given another actor, and the same act is refused by name with the
# journal's line count unchanged.
ia_conflict() {
    local name=$1 type=$2 lines out status
    shift 2
    cp "$IA_J" "$IP_TMP/journal.saved"
    python3 - "$IA_J" "$type" <<'PY'
import json, sys
path, kind = sys.argv[1], sys.argv[2]
lines = open(path).read().splitlines()
hit = [i for i, l in enumerate(lines) if json.loads(l)["type"] == kind]
if not hit:
    sys.exit(9)
e = json.loads(lines[hit[-1]]); e["actor_ref"] = "person://Somebody Else"
lines[hit[-1]] = json.dumps(e, separators=(",", ":"))
open(path, "w").write("\n".join(lines) + "\n")
PY
    if [[ $? -ne 0 ]]; then
        printf 'PLANT MUTATION WAS A NO-OP: no %s event in %s\n' "$type" "$IA_J" >&2
        exit 9
    fi
    lines=$(wc -l < "$IA_J")
    out=$(PATH="$IP_TMP/bin:$PATH" ia "$@" </dev/null 2>&1); status=$?
    if [[ $status -ne 0 ]] && grep -q 'journal.idempotency-conflict' <<<"$out" && grep -q 'Somebody Else' <<<"$out" \
        && [[ $(wc -l < "$IA_J") -eq $lines ]]; then
        ip_ok "$name" "journal.idempotency-conflict, $lines line(s) unchanged"
    else
        ip_fail "$name" "exit $status, lines $lines → $(wc -l < "$IA_J"): $(ip_errors "$out")"
    fi
    cp "$IP_TMP/journal.saved" "$IA_J"
}

# The single-act ingest `war sign` and the batch share: signed once, then the
# same response ingested again.
IA_OUT=$(ia sign "$IA_A" --ssh-sign --as "Plant Signer" </dev/null 2>&1)
IA_RESP=$(ls "$IA"/docs/authority/responses/"$IA_A".*response.toml 2>/dev/null | head -1)
if [[ -n "$IA_RESP" ]] && [[ -f "$IA_DIR/authorization.toml" ]]; then
    ip_ok "war sign records once" "$(basename "$IA_RESP")"
else
    ip_fail "war sign records once" "$(ip_errors "$IA_OUT")"
fi
ia_retry "a signed act retried replays" "authorize.replayed" authorize "$IA_A" --response "$IA_RESP"
ia_conflict "a signed act by another refuses" "authorization.recorded" authorize "$IA_A" --response "$IA_RESP"

# war ask.
ia ask "$IA_A" STAGE-001 "Which gate runs the battery?" --recommend "ops.conformance" >/dev/null 2>&1
ia_retry "war ask retried replays" "question.replayed" ask "$IA_A" STAGE-001 "Which gate runs the battery?" --recommend "ops.conformance"
ia_conflict "war ask by another refuses" "question.asked" ask "$IA_A" STAGE-001 "Which gate runs the battery?" --recommend "ops.conformance"
# The control: a different question is not a retry.
IA_OUT=$(ia ask "$IA_A" STAGE-001 "A different question?" 2>&1)
if grep -qE 'PASS +question\.asked' <<<"$IA_OUT" && ! grep -q 'replayed' <<<"$IA_OUT"; then
    ip_ok "a different question is asked" "question.asked, not replayed"
else
    ip_fail "a different question is asked" "$(ip_errors "$IA_OUT")"
fi

# war submit, answering a dispatch this Warrant compiled.
ia dispatch "$IA_A" STAGE-002 >/dev/null 2>&1
IA_DID=$(python3 -c 'import json,sys
for l in open(sys.argv[1]):
    e = json.loads(l)
    if e["type"] == "dispatch.compiled": d = json.loads(e["payload"])["dispatch_id"]
print(d)' "$IA_J" 2>/dev/null)
python3 - "$IA_DID" "$IP_TMP/submission.json" <<'PY'
import json, sys
json.dump({"dispatch_id": sys.argv[1], "attempt_id": "plant-attempt-1", "contract_digest": "sha256:plant",
           "stage_id": "STAGE-002", "requested_next_action": "verify"}, open(sys.argv[2], "w"))
PY
IA_OUT=$(ia submit "$IA_A" "$IP_TMP/submission.json" 2>&1)
if grep -qE 'PASS +submission\.recorded' <<<"$IA_OUT"; then
    ip_ok "war submit records once" "dispatch $IA_DID"
else
    ip_fail "war submit records once" "$(ip_errors "$IA_OUT")"
fi
ia_retry "war submit retried replays" "submission.replayed" submit "$IA_A" "$IP_TMP/submission.json"
ia_conflict "war submit by another refuses" "submission.recorded" submit "$IA_A" "$IP_TMP/submission.json"

# war verify --response, from a verifier who is not the performer.
cat > "$IP_TMP/verification.toml" <<VERIFY
schema = "oh.war/verification-response/v1"
warrant = "$IA_A"

[[verifications]]
obligation = "OBL-001"
disposition = "established"
evidence = "the plant's fixture evidence for OBL-001"
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
IA_OUT=$(ia verify "$IA_A" --response "$IP_TMP/verification.toml" 2>&1)
if grep -qE 'PASS +verify\.recorded' <<<"$IA_OUT" && [[ -f "$IA_DIR/verifications/OBL-001.toml" ]]; then
    ip_ok "war verify records once" "OBL-001 established by plant-verifier"
else
    ip_fail "war verify records once" "$(ip_errors "$IA_OUT")"
fi
cp "$IA_DIR/verifications/OBL-001.toml" "$IP_TMP/verification-record.toml" 2>/dev/null
ia_retry "war verify retried replays" "verify.replayed" verify "$IA_A" --response "$IP_TMP/verification.toml"
ia_conflict "war verify by another refuses" "verification.recorded" verify "$IA_A" --response "$IP_TMP/verification.toml"
# The negative control: the binary from before this change, on the same retry.
if [[ -n "${IP_WAR_BEFORE:-}" && -x "${IP_WAR_BEFORE:-}" ]]; then
    IA_BEFORE_TREE=$(ip_tree "$IA_DIR")
    IA_MTIME=$(stat -c %y "$IA_DIR/verifications/OBL-001.toml")
    sleep 0.05
    IA_OUT=$("$IP_WAR_BEFORE" --root "$IA" verify "$IA_A" --response "$IP_TMP/verification.toml" 2>&1); IA_STATUS=$?
    if [[ $IA_STATUS -ne 0 ]] && grep -q 'already journalled' <<<"$IA_OUT" \
        && [[ "$(stat -c %y "$IA_DIR/verifications/OBL-001.toml")" != "$IA_MTIME" ]]; then
        ip_ok "control: before, the retry failed" "exit $IA_STATUS after rewriting the record: already journalled"
    else
        ip_fail "control: before, the retry failed" "exit $IA_STATUS: $(ip_errors "$IA_OUT")"
    fi
    [[ "$(ip_tree "$IA_DIR")" == "$IA_BEFORE_TREE" ]] || git -C "$IA" checkout -q -- . 2>/dev/null
else
    printf 'note  %-34s %s\n' "control: before, the retry failed" "not run: set IP_WAR_BEFORE to a war built before OW-WAR-0130"
fi

# war evidence record: the program's gate (`war check --generated`) once, then
# the same act again. The projections are brought up to date first, so the
# gate passes and its run is admissible.
ia compile >/dev/null 2>&1
IA_OUT=$(PATH="$IP_TMP/bin:$PATH" ia evidence record "$IA_A" 2>&1)
if grep -qE 'PASS +gate-run\.receipt|receipt' <<<"$IA_OUT" && ls "$IA_DIR"/gate-runs/*.receipt.json >/dev/null 2>&1; then
    ip_ok "war evidence record records once" "$(ls "$IA_DIR"/gate-runs/*.receipt.json | wc -l) receipt(s)"
else
    ip_fail "war evidence record records once" "$(ip_errors "$IA_OUT")"
fi
ia_retry "war evidence retried replays" "evidence.replayed" evidence record "$IA_A"
ia_conflict "war evidence by another refuses" "sync.receipt_attached" evidence record "$IA_A"
corpus_gone "$IA"
unset PLANT_ROOT

# ── OBL-003: an older war refuses a newer repository; a newer record is UNKNOWN
PLANT_ROOT=$(ip_corpus IV)
IV=$PLANT_ROOT
iv() { "$WAR" --root "$IV" "$@"; }
IV_VERSION=$("$WAR" --version | awk '{print $2}')
iv_requires() { # requirement → openwarrant.toml carries it under [project]
    git -C "$IV" checkout -q -- openwarrant.toml
    python3 - "$IV/openwarrant.toml" "$1" <<'PY'
import sys
p, req = sys.argv[1], sys.argv[2]
s = open(p).read()
s = s.replace("[project]\n", f'[project]\nrequires_war = "{req}"\n', 1)
open(p, "w").write(s)
PY
    assert_present "requires_war = \"$1\"" "$IV/openwarrant.toml"
}
IV_BASE=$(iv status 2>&1); IV_BASE_STATUS=$?

iv_requires ">=99"
IV_OUT=$(iv status 2>&1); IV_STATUS=$?
if [[ $IV_STATUS -ne 0 ]] && grep -q 'compat.war-too-old' <<<"$IV_OUT" && grep -qF '>=99' <<<"$IV_OUT" \
    && grep -qF "war $IV_VERSION" <<<"$IV_OUT" && ! grep -q 'IV-WAR-' <<<"$IV_OUT"; then
    ip_ok "requires_war >=99 refuses" "compat.war-too-old (exit $IV_STATUS), >=99 and war $IV_VERSION named, no Warrant read"
else
    ip_fail "requires_war >=99 refuses" "exit $IV_STATUS: $(head -3 <<<"$IV_OUT" | tr '\n' '|')"
fi
iv_requires "=$IV_VERSION"
IV_OUT=$(iv status 2>&1); IV_STATUS=$?
if [[ $IV_STATUS -eq $IV_BASE_STATUS ]] && [[ "$IV_OUT" == "$IV_BASE" ]] && ! grep -q 'compat\.' <<<"$IV_OUT"; then
    ip_ok "requires_war = this war passes" "=$IV_VERSION: status unchanged from no key (exit $IV_STATUS)"
else
    ip_fail "requires_war = this war passes" "exit $IV_STATUS vs $IV_BASE_STATUS: $(diff <(echo "$IV_BASE") <(echo "$IV_OUT") | head -3 | tr '\n' '|')"
fi
iv_requires "not a version"
IV_OUT=$(iv status 2>&1); IV_STATUS=$?
if [[ $IV_STATUS -ne 0 ]] && grep -q 'requires_war' <<<"$IV_OUT" && grep -q 'not a version requirement' <<<"$IV_OUT"; then
    ip_ok "a malformed requires_war refuses" "not a version requirement (exit $IV_STATUS)"
else
    ip_fail "a malformed requires_war refuses" "exit $IV_STATUS: $(head -2 <<<"$IV_OUT" | tr '\n' '|')"
fi
git -C "$IV" checkout -q -- openwarrant.toml

# A verification record written by a newer war: UNKNOWN, never PASS or ERROR.
mkdir -p "$IV/docs/warrants/IV-WAR-0001/verifications"
IV_REC="$IV/docs/warrants/IV-WAR-0001/verifications/OBL-001.toml"
{ echo 'schema = "oh.war/verification/v1"'; cat "$IP_TMP/verification-record.toml"; } > "$IV_REC"
IV_OUT=$(iv check 2>&1)
IV_ERRORS_V1=$(grep -c '^ERROR' <<<"$IV_OUT")
if grep -q 'compat.newer-record' <<<"$IV_OUT"; then
    ip_fail "a v1 record is not newer" "$(grep 'compat.newer-record' <<<"$IV_OUT" | head -1)"
else
    ip_ok "a v1 record is not newer" "compat.newer-record silent"
fi
sed -i 's|oh.war/verification/v1|oh.war/verification/v9|' "$IV_REC"
assert_present 'oh.war/verification/v9' "$IV_REC"
IV_OUT=$(iv check 2>&1)
IV_LINE=$(grep -A1 'compat.newer-record' <<<"$IV_OUT" | tr '\n' ' ')
if grep -qE '^UNKNOWN +compat\.newer-record' <<<"$IV_OUT" && grep -q 'verifications/OBL-001.toml' <<<"$IV_LINE" \
    && grep -q 'v9' <<<"$IV_LINE" && ! grep -E '^(PASS|ERROR)' <<<"$IV_OUT" | grep -q 'compat.newer-record' \
    && [[ $(grep -c '^ERROR' <<<"$IV_OUT") -eq $IV_ERRORS_V1 ]]; then
    ip_ok "a v9 record is UNKNOWN" "UNKNOWN compat.newer-record, no PASS, no new ERROR ($IV_ERRORS_V1)"
else
    ip_fail "a v9 record is UNKNOWN" "$(grep -E 'compat|^ERROR' <<<"$IV_OUT" | head -3 | tr '\n' '|')"
fi
corpus_gone "$IV"
unset PLANT_ROOT

ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$IP_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$IP_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
unset SSH_AGENT_PID
command rm -rf "$IP_TMP"
