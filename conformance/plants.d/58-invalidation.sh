# shellcheck shell=bash
# OW-WAR-0136 — the Phase 9 exit: custody audited, invalidation that
# propagates, and no step by the actor who produced the work.
#
# A scratch program (IV) with five Warrants, built through the real acts by a
# key generated here and held by a throwaway ssh-agent that holds nothing
# else; nothing here is the owner's key or this repository's corpus:
#
#   A IV-WAR-0001  resolved, rests on gate G (plant.g@1.0.0)
#   B IV-WAR-0002  resolved, rests on G
#   C IV-WAR-0003  resolved, rests on gate H (plant.h@1.0.0); A's child (§20.2)
#   D IV-WAR-0004  resolved, rests only on H — the control
#   E IV-WAR-0005  authorized, evidenced, verified, NOT resolved; cites G
#
# Receipts as resolve-attestation subjects, `war attest --custody` and its
# record, the invalidation request with §45's sweep, and — Q-001 (a) — the
# invalidate act: signed with `war sign` by a human holding `resolver`, it
# records the invalidation and one §56.4 dispute per reached resolution;
# `war check` reads standing from the disputes; the gate's receipt stops
# counting for requirement 5 on the unresolved Warrant. An agent, the
# performer and an unsigned response are refused and write nothing, and an
# invalidation whose signature is gone counts for nothing.
#
# Every claim is paired with the control seen refusing, matched by rule.

echo "== Phase 9 exit: custody and invalidation (OW-WAR-0136) =="
PLANT_ROOT=$(scratch_corpus IV)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
IV_TMP=$(mktemp -d)

# ── Setup, under -euo pipefail: any failing step aborts it, so no plant below
# scores a half-built corpus. The agent lives only as long as the setup.
if ! WAR="$REPO_ROOT/$WAR" D="$PLANT_ROOT" T="$IV_TMP" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$IV_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
for gid in plant.g plant.h; do
cat > "docs/gates/$gid@1.0.0.yaml" <<GATE
gate_id: "$gid"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "artifact://coreutils/true"
output_schema_ref: "schema://exit-status/v1"
provenance: "local_candidate"
input_kinds: ["warrant-corpus"]
inputs: ["src/**"]
argv: ["true"]
mutating: "false"
timeout_secs: "5"
fault_model: ["plant-cannot-run"]
known_blind_spots: ["Passes whenever true exits 0; it stands in for a gate."]
qualification_qualifier: "conformance/plants.d/58-invalidation.sh"
qualification_digest: ""
qualification_positive_controls: ["The plant's Warrants resolve with it."]
qualification_negative_controls: ["None; it qualifies the seam, not any work."]
qualification_mutation_classes: ["none"]
qualification_environments: ["linux-x86_64"]
qualification_limitations: ["A gate that always passes qualifies nothing but its own execution."]
detection_results:
  - fault_class: "plant-cannot-run"
    mutation: "none"
    detected: "true"
GATE
done
# mkw <alias> <gate> [parent-alias]: a Warrant citing one gate, one deliverable.
mkw() {
    local a="$1" gate="$2" parent="${3:-}" w="docs/warrants/$1"
    if [[ "$a" != IV-WAR-0001 ]]; then
        mkdir -p "$w"
        cp -r docs/warrants/IV-WAR-0001/atoms "$w/atoms"
        python3 - "docs/warrants/IV-WAR-0001/manifest.toml" "$w/manifest.toml" "$a" <<'PY'
import sys, re, uuid, time
src, dst, alias = sys.argv[1:4]
s = open(src).read()
ms = int(time.time() * 1000)
r = uuid.uuid4().hex
u = f"{ms:012x}"[:8] + "-" + f"{ms:012x}"[8:12] + "-7" + r[:3] + "-" + "8" + r[3:6] + "-" + r[6:18]
s = re.sub(r'(?m)^uuid = ".*"', f'uuid = "{u}"', s)
s = re.sub(r'(?m)^local_alias = ".*"', f'local_alias = "{alias}"', s)
s = re.sub(r'(?m)^title = ".*"', f'title = "Plant Warrant {alias}"', s)
open(dst, "w").write(s)
old = re.search(r'(?m)^uuid = "(.*)"', open(src).read()).group(1)
import glob, os
for a in glob.glob(os.path.join(os.path.dirname(dst), "atoms", "*")):
    t = open(a).read().replace(old, u)
    open(a, "w").write(t)
PY
    fi
    python3 - "$w/atoms/60-assurance.md" "$gate" <<'PY'
import sys
p, gate = sys.argv[1], sys.argv[2]; s = open(p).read()
s = s.split("## Acceptance Obligations")[0] + f"""## Acceptance Obligations

### OBL-001 — the scratch file says resolved
- **scope:** one file in this scratch program.
- **gate:** `gate://{gate}`
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
    printf '%s\n' "$a" > "src/$a.txt"
    cat > "$w/deliverables.toml" <<DELIV
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "the plant file"
kind = "file"
target_ref = "src/$a.txt"
required = true
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001"]

[deliverable.provenance]
producer = "claude"
producing_attempt = "$a/attempt-1"
contract_digest = "unrecorded"
tool_or_runtime_identity = "plant"
creation_method = "authored"
content_digest = "sha256:$(sha256sum "src/$a.txt" | cut -d' ' -f1)"
media_type = "text/plain"
classification = "internal"
retention = "repository-lifetime"
source_holder = "git"
DELIV
    printf 'schema = "oh.war/rationale/v1"\n' > "$w/rationale.toml"
    if [[ -n "$parent" ]]; then
        local pu pd
        pu=$(sed -n 's/^uuid = "\(.*\)"/\1/p' "docs/warrants/$parent/manifest.toml")
        pd=$("$WAR" --root . authorize "$parent" --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["contract_digest"])')
        printf '\n[[parents]]\nref = "war://%s"\ncontract_revision = 1\ncontract_digest = "sha256:%s"\n' "$pu" "${pd#sha256:}" >> "$w/manifest.toml"
    fi
}
mkw IV-WAR-0001 plant.g@1.0.0
mkw IV-WAR-0002 plant.g@1.0.0
mkw IV-WAR-0003 plant.h@1.0.0 IV-WAR-0001
mkw IV-WAR-0004 plant.h@1.0.0
mkw IV-WAR-0005 plant.g@1.0.0
ssh-keygen -q -t ed25519 -N "" -C plant -f "$T/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$T/id_plant.pub")" > docs/authority/allowed_signers
cat > docs/authority/roles.toml <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/58-invalidation.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the invalidation plants run."
ssh_principal = "plant"
ROLES
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "five Warrants: four to resolve, one left unresolved"

eval "$(ssh-agent -s)" >/dev/null
trap 'ssh-agent -k >/dev/null 2>&1 || true' EXIT
ssh-add -q "$T/id_plant"
keys=$(ssh-add -l)
fp=$(ssh-keygen -lf "$T/id_plant.pub" | awk '{print $2}')
[[ $(grep -c . <<<"$keys") -eq 1 ]] && grep -qF -- "$fp" <<<"$keys" \
    || { echo "the throwaway agent holds more than the plant key: $keys"; exit 1; }
for a in IV-WAR-0001 IV-WAR-0002 IV-WAR-0003 IV-WAR-0004 IV-WAR-0005; do
    "$WAR" --root . sign "$a" --ssh-sign --as "Plant Signer" </dev/null
    g add -A; g commit -qm "authorized $a"
    "$WAR" --root . evidence record "$a"
    g add -A; g commit -qm "evidence $a"
    cat > "$T/verified-$a.toml" <<VERIFY
schema = "oh.war/verification-response/v1"
warrant = "$a"

[[verifications]]
obligation = "OBL-001"
disposition = "established"
evidence = "src/$a.txt read"
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
    "$WAR" --root . verify "$a" --response "$T/verified-$a.toml"
    g add -A; g commit -qm "verified $a"
    [[ "$a" == IV-WAR-0005 ]] && continue
    "$WAR" --root . sign "$a" --ssh-sign --as "Plant Signer" </dev/null
    test -f "docs/warrants/$a/resolution.toml"
    "$WAR" --root . compile >/dev/null
    g add -A; g commit -qm "resolved $a"
done
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the resolved scratch corpus:\n' >&2
    tail -15 "$IV_TMP/setup.log" >&2
    exit 9
fi

IV_A=IV-WAR-0001; IV_B=IV-WAR-0002; IV_C=IV-WAR-0003; IV_D=IV-WAR-0004; IV_E=IV-WAR-0005
IV_W="$PLANT_ROOT/docs/warrants"
IV_RECEIPT="docs/warrants/$IV_B/gate-runs/plant_g_1_0_0.receipt.json"
iv_ok()      { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
iv_fail()    { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# Law 15: a case this Warrant could not build — its ingest waits on Q-001 — is
# said to be unknown, and counted as neither.
iv_unknown() { printf 'UNKNOWN %-32s %s\n' "$1" "$2"; }
iv_war()     { "$WAR" --root "$PLANT_ROOT" "$@"; }
iv_lines()   { grep -E '^(PASS|ERROR|UNKNOWN|WARN)' <<<"$1" | head -3 | tr '\n' '|'; }
# A digest of every record the obligations say is never rewritten: each
# resolution, receipt, run, output, attestation, and the Gate Definitions.
iv_records() {
    (cd "$PLANT_ROOT" && find docs/warrants docs/gates -type f \
        \( -name resolution.toml -o -path '*/gate-runs/*' -o -path '*/attestations/*' -o -path 'docs/gates/*' \) \
        -print0 | sort -z | xargs -0 sha256sum | sha256sum)
}
iv_clean() { [[ -z "$(git -C "$PLANT_ROOT" status --porcelain)" ]]; }

# ---------------------------------------------------------------- OBL-001 ---
# The resolve attestation names every relied-on file, each at its digest on
# disk — recomputed here from the files, not read back from the tool.
if python3 - "$PLANT_ROOT" "$IV_B" <<'PY'
import base64, hashlib, json, sys, tomllib
root, alias = sys.argv[1], sys.argv[2]
env = json.load(open(f"{root}/docs/warrants/{alias}/attestations/resolve-1.dsse.json"))
st = json.loads(base64.b64decode(env["payload"]))
subj = {s["name"]: s["digest"]["sha256"] for s in st["subject"]}
res = tomllib.load(open(f"{root}/docs/warrants/{alias}/resolution.toml", "rb"))
refs = res["resolution"]["gate_run_refs"]
if not refs:
    sys.exit(1)
for r in refs:
    rec = json.load(open(f"{root}/{r}"))
    for f in (r, r.replace(".receipt.json", ".run.toml"), rec["stdout_ref"], rec["stderr_ref"]):
        if subj.get(f) != hashlib.sha256(open(f"{root}/{f}", "rb").read()).hexdigest():
            sys.exit(1)
PY
then
    iv_ok "receipts are attestation subjects" "receipt, run, stdout, stderr each at its digest on disk"
else
    iv_fail "receipts are attestation subjects" "the resolve attestation of $IV_B does not name every relied-on file at its digest"
fi

# Control: untouched, both pass.
IV_V=$(iv_war attest "$IV_B" --verify 2>&1); IV_VS=$?
IV_C_OUT=$(iv_war attest "$IV_B" --custody 2>&1); IV_CS=$?
if [[ $IV_VS -eq 0 && $IV_CS -eq 0 ]] && grep -q 'attest.verified' <<<"$IV_V" \
    && grep -q 'attest.custody-audited' <<<"$IV_C_OUT" && ! grep -q '^ERROR' <<<"$IV_C_OUT$IV_V"; then
    iv_ok "untouched, verify and custody pass" "attest.verified, attest.custody-audited (exit 0, 0)"
else
    iv_fail "untouched, verify and custody pass" "exit $IV_VS/$IV_CS: $(iv_lines "$IV_V") $(iv_lines "$IV_C_OUT")"
fi

# The attack: the receipt replaced by a freshly minted one that reseals over
# its own fields, so every check that reads the seal still passes.
python3 - "$PLANT_ROOT/$IV_RECEIPT" <<'PY'
import hashlib, json, sys
p = sys.argv[1]; r = json.load(open(p))
r["completed_at"] = "2099-01-01T00:00:00Z"
u = dict(r, receipt_digest="")
pre = json.dumps({"digest_domain": "oh.war/gate-run/v1", "payload": u},
                 sort_keys=True, separators=(",", ":"), ensure_ascii=False)
r["receipt_digest"] = "sha256:" + hashlib.sha256(pre.encode()).hexdigest()
open(p, "w").write(json.dumps(r, indent=2) + "\n")
PY
assert_present '2099-01-01T00:00:00Z' "$PLANT_ROOT/$IV_RECEIPT"
IV_CHECK=$(iv_war check "$IV_B" 2>&1)
IV_V=$(iv_war attest "$IV_B" --verify 2>&1); IV_VS=$?
IV_C_OUT=$(iv_war attest "$IV_B" --custody 2>&1); IV_CS=$?
if grep -q 'receipt-invalid' <<<"$IV_CHECK"; then
    iv_fail "the replacement reseals" "war check calls the replaced receipt invalid; the plant did not reseal it"
elif [[ $IV_VS -ne 0 ]] && grep 'attest.subject-drift' <<<"$IV_V" | grep -qF "$IV_RECEIPT"; then
    iv_ok "a replaced receipt fails --verify" "attest.subject-drift names $IV_RECEIPT (exit $IV_VS)"
else
    iv_fail "a replaced receipt fails --verify" "exit $IV_VS: $(iv_lines "$IV_V")"
fi
if [[ $IV_CS -ne 0 ]] && grep -E '^ERROR +attest\.custody-drift' <<<"$IV_C_OUT" | grep -qF "$IV_RECEIPT"; then
    iv_ok "a replaced receipt fails --custody" "attest.custody-drift names $IV_RECEIPT (exit $IV_CS)"
else
    iv_fail "a replaced receipt fails --custody" "exit $IV_CS: $(iv_lines "$IV_C_OUT")"
fi
git -C "$PLANT_ROOT" checkout -q -- "$IV_RECEIPT"

# ---------------------------------------------------------------- OBL-002 ---
# Nine §41.5 fields per receipt, each present with a value or UNKNOWN with a
# reason, never both; access history UNKNOWN because git records no reads.
iv_fields() { # alias → exit 0 when the audit's fields are well formed; prints original digest's status
    "$WAR" "$@" --json 2>/dev/null | python3 -c '
import json, sys
want = ["collector", "original digest", "transfer method", "storage event",
        "instrument or runner identity", "calibration or qualification",
        "transformations", "access history", "derivative lineage"]
r = json.loads(sys.stdin.read())["result"]
out = []
for rc in r["receipts"]:
    fs = rc["fields"]
    if [f["field"] for f in fs] != want: sys.exit(1)
    for f in fs:
        if f["status"] == "present" and (not f.get("value") or "reason" in f): sys.exit(1)
        if f["status"] == "unknown" and (not f.get("reason") or "value" in f): sys.exit(1)
        if f["status"] not in ("present", "unknown"): sys.exit(1)
    ah = next(f for f in fs if f["field"] == "access history")
    if ah["status"] != "unknown" or ah["reason"] != "git records no reads": sys.exit(1)
    od = next(f for f in fs if f["field"] == "original digest")
    out.append(od["status"] + ":" + od.get("reason", ""))
if not out: sys.exit(1)
print(",".join(out))'
}
IV_OD=$(iv_fields --root "$PLANT_ROOT" attest "$IV_A" --custody); IV_FS=$?
if [[ $IV_FS -eq 0 && "$IV_OD" == "present:" ]]; then
    iv_ok "nine fields, scratch resolution" "each present or UNKNOWN with a reason; original digest present"
else
    iv_fail "nine fields, scratch resolution" "fields malformed or original digest $IV_OD"
fi
# The same audit on a resolution signed before receipts were subjects: the
# original digest is UNKNOWN (not attested), not present. Read-only here.
IV_OLD=OW-WAR-0062
if [[ -f "$REPO_ROOT/docs/warrants/$IV_OLD/resolution.toml" ]]; then
    IV_OD=$(iv_fields --root "$REPO_ROOT" attest "$IV_OLD" --custody); IV_FS=$?
    IV_OUT=$("$WAR" --root "$REPO_ROOT" attest "$IV_OLD" --custody 2>&1)
    if [[ $IV_FS -eq 0 && "$IV_OD" == "unknown:not attested" ]] && grep -qF 'original digest: UNKNOWN (not attested)' <<<"$IV_OUT" \
        && ! grep -qE '^PASS .*original digest' <<<"$IV_OUT"; then
        iv_ok "an old resolution is not attested" "$IV_OLD: original digest UNKNOWN (not attested), not present"
    else
        iv_fail "an old resolution is not attested" "$IV_OLD: original digest '$IV_OD'"
    fi
else
    iv_unknown "an old resolution is not attested" "$IV_OLD carries no resolution in this checkout"
fi
# --record by the performer: SelfAct, nothing written. Then the control: an
# auditor who is not the performer records it.
IV_BEFORE=$(iv_records)
IV_OUT=$(iv_war attest "$IV_A" --custody --record claude 2>&1); IV_S=$?
if [[ $IV_S -ne 0 ]] && grep -E '^ERROR +attest\.custody-self-act' <<<"$IV_OUT" | grep -q 'SelfAct' \
    && [[ ! -e "$IV_W/$IV_A/custody-audit.toml" ]] && iv_clean && [[ "$(iv_records)" == "$IV_BEFORE" ]]; then
    iv_ok "the performer may not record it" "attest.custody-self-act (SelfAct); nothing written"
else
    iv_fail "the performer may not record it" "exit $IV_S: $(iv_lines "$IV_OUT"); $(git -C "$PLANT_ROOT" status --porcelain | head -2 | tr '\n' ' ')"
fi
IV_OUT=$(iv_war attest "$IV_A" --custody --record plant-verifier 2>&1); IV_S=$?
if [[ $IV_S -eq 0 ]] && grep -q 'attest.custody-recorded' <<<"$IV_OUT" \
    && grep -q '^auditor = "plant-verifier"' "$IV_W/$IV_A/custody-audit.toml" 2>/dev/null \
    && grep -q '^verdict = "intact"' "$IV_W/$IV_A/custody-audit.toml"; then
    iv_ok "another auditor records it" "custody-audit.toml names plant-verifier, verdict intact"
else
    iv_fail "another auditor records it" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi
command rm -f "$IV_W/$IV_A/custody-audit.toml"

# ---------------------------------------------------------------- OBL-003 ---
# The sweep: invalidating plant.g reaches A and B through their receipts and C
# through its parent A; D rests only on plant.h and is not reached.
iv_sweep() { # gate → "disputed|unaffected|C's via"
    "$WAR" --root "$PLANT_ROOT" gate invalidate "$1" --grounds "the plant's grounds" --json 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["result"]
via = {d["warrant"]: d["via"] for d in r["disputes"]}
print(",".join(sorted(via)) + "|" + ",".join(sorted(r["unaffected"])) + "|" + ";".join(via.get("IV-WAR-0003", [])))'
}
IV_BEFORE=$(iv_records)
IV_SW=$(iv_sweep plant.g@1.0.0)
if [[ "$IV_SW" == "$IV_A,$IV_B,$IV_C|$IV_D|parent $IV_A" ]]; then
    iv_ok "invalidating G reaches A, B, C" "C through its parent A; D unaffected"
else
    iv_fail "invalidating G reaches A, B, C" "sweep $IV_SW"
fi
# Control: a sweep that disputed everything, or ignored the gate, fails here.
IV_SW=$(iv_sweep plant.h@1.0.0)
if [[ "$IV_SW" == "$IV_C,$IV_D|$IV_A,$IV_B|receipt docs/warrants/$IV_C/gate-runs/plant_h_1_0_0.receipt.json" ]]; then
    iv_ok "invalidating H reaches C, D only" "A and B, which rest on G, are left standing"
else
    iv_fail "invalidating H reaches C, D only" "sweep $IV_SW"
fi

# ---------------------------------------------------------------- OBL-004 ---
# The request rewrites nothing; a refused request writes nothing either.
IV_OUT=$(iv_war gate invalidate plant.g@1.0.0 --grounds "" 2>&1); IV_S1=$?
IV_OUT2=$(iv_war gate invalidate plant.g --grounds "g" 2>&1); IV_S2=$?
if iv_clean && [[ "$(iv_records)" == "$IV_BEFORE" ]]; then
    iv_ok "the request rewrites nothing" "every resolution, receipt, run, attestation and G byte-identical"
else
    iv_fail "the request rewrites nothing" "$(git -C "$PLANT_ROOT" status --porcelain | head -3 | tr '\n' ' ')"
fi
if [[ $IV_S1 -ne 0 && $IV_S2 -ne 0 ]] && grep -q 'invalidation.no-grounds' <<<"$IV_OUT" \
    && grep -q 'invalidation.gate-ref' <<<"$IV_OUT2" && iv_clean; then
    iv_ok "no grounds or no version: refused" "invalidation.no-grounds, invalidation.gate-ref"
else
    iv_fail "no grounds or no version: refused" "exit $IV_S1/$IV_S2: $(iv_lines "$IV_OUT") $(iv_lines "$IV_OUT2")"
fi

# ---------------------------------------------------------------- OBL-005 ---
# The custody audit by the performer is refused above. Unchanged from today:
# a resolution by the performer, registered as a human resolver, is SelfAct.
cat >> "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'

[[assignment]]
actor = "claude"
actor_kind = "human"
roles = ["resolver"]
assigned_by = "conformance/plants.d/58-invalidation.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "The performer registered as a human resolver, so the refusal seen is SelfAct and not the agent rule."
ROLES
iv_response() { # resolver → response file
    cat > "$IV_TMP/resolve-$1.toml" <<RESP
schema = "oh.war/resolution-response/v1"
warrant = "$IV_E"
contract_digest = "$(iv_war authorize "$IV_E" --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["contract_digest"])')"
resolved_by = "$1"
acting_role = "resolver"
common_outcome = "satisfied"
profile_outcome = "delivered"
meaning = "The plant's resolution of $IV_E."
effective_time = "2026-09-24T00:00:00Z"
RESP
    printf '%s' "$IV_TMP/resolve-$1.toml"
}
IV_OUT=$(iv_war resolve "$IV_E" --response "$(iv_response claude)" 2>&1); IV_S=$?
if [[ $IV_S -ne 0 ]] && grep -E '^ERROR +resolution\.not-permitted' <<<"$IV_OUT" | grep -q 'performed this work' \
    && [[ ! -e "$IV_W/$IV_E/resolution.toml" ]]; then
    iv_ok "the performer may not resolve" "resolution.not-permitted (SelfAct); no resolution.toml"
else
    iv_fail "the performer may not resolve" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi
# Control: the same Warrant is resolvable by someone else — the refusal is the
# actor, not an unmet requirement.
IV_OUT=$(iv_war resolve "$IV_E" --dry-run 2>&1); IV_S=$?
if [[ $IV_S -eq 0 ]] && ! grep -q 'requirement-unmet' <<<"$IV_OUT"; then
    iv_ok "the same Warrant is resolvable" "the thirteen hold for $IV_E"
else
    iv_fail "the same Warrant is resolvable" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi

# ------------------------------------------ the invalidate act (Q-001 a) ---
# From here the records change, so the tree is committed first: "writes
# nothing" is read from git status. The register gains an agent holding
# `resolver`, so the refusal seen for it is by kind, not by role.
cat >> "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'

[[assignment]]
actor = "plant-agent"
actor_kind = "agent"
roles = ["resolver"]
assigned_by = "conformance/plants.d/58-invalidation.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "An agent holding resolver, so an invalidation it claims is refused by kind."
ROLES
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qam "the performer and an agent as resolvers"
IV_G=plant.g@1.0.0
IV_GROUNDS="plant.g passes whenever true exits 0; it cannot fail, so it evidences nothing."
IV_INV="$PLANT_ROOT/docs/gates/invalidations/$IV_G.toml"
iv_no_disputes() { ! compgen -G "$IV_W/*/disputes" >/dev/null && [[ ! -e "$IV_INV" ]]; }
# A hand-written response, as a human or an agent would hand one in.
iv_inv_response() { # actor → file
    iv_war gate invalidate "$IV_G" --grounds "$IV_GROUNDS" --json 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["result"]
actor, grounds = sys.argv[1], sys.argv[2]
q = json.dumps
print("schema = \"oh.war/invalidation-response/v1\"")
print("gate = " + q(r["gate"]))
print("definition_digest = " + q(r["definition_digest"]))
print("grounds = " + q(grounds))
print("disputes = [" + ", ".join(q(d["warrant"]) for d in r["disputes"]) + "]")
print("invalidated_by = " + q(actor))
print("acting_role = \"resolver\"")
print("meaning = \"written by hand in the plant\"")
print("effective_time = \"2026-09-25T00:00:00Z\"")' "$1" "$IV_GROUNDS" > "$IV_TMP/invalidate-$1.toml"
    printf '%s' "$IV_TMP/invalidate-$1.toml"
}

# Control, before: E's receipt of G counts, and nothing is disputed.
IV_OUT=$(iv_war check "$IV_E" 2>&1)
if grep -E '^PASS +evidence\.admissible' <<<"$IV_OUT" | grep -qF "$IV_G" \
    && ! grep -q 'resolution.disputed' <<<"$(iv_war check 2>&1)"; then
    iv_ok "before: G's receipt counts on E" "evidence.admissible; no resolution disputed"
else
    iv_fail "before: G's receipt counts on E" "$(iv_lines "$IV_OUT")"
fi

# OBL-005: an agent's invalidation is refused by kind, the performer's with
# SelfAct, and an unsigned one by the eligible resolver is refused too — each
# before anything is written.
IV_BEFORE=$(iv_records)
IV_OUT=$(iv_war gate invalidate "$IV_G" --response "$(iv_inv_response plant-agent)" 2>&1); IV_S=$?
if [[ $IV_S -ne 0 ]] && grep -E '^ERROR +invalidation\.agent' <<<"$IV_OUT" | grep -q '§27.2' \
    && iv_clean && iv_no_disputes; then
    iv_ok "an agent's invalidation: refused" "invalidation.agent (§27.2, by kind); nothing written"
else
    iv_fail "an agent's invalidation: refused" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi
IV_OUT=$(iv_war gate invalidate "$IV_G" --response "$(iv_inv_response claude)" 2>&1); IV_S=$?
if [[ $IV_S -ne 0 ]] && grep -E '^ERROR +invalidation\.self-act' <<<"$IV_OUT" | grep -q 'SelfAct' \
    && iv_clean && iv_no_disputes; then
    iv_ok "the performer's invalidation: refused" "invalidation.self-act (SelfAct); nothing written"
else
    iv_fail "the performer's invalidation: refused" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi
IV_OUT=$(iv_war gate invalidate "$IV_G" --response "$(iv_inv_response "Plant Signer")" 2>&1); IV_S=$?
if [[ $IV_S -ne 0 ]] && grep -E '^ERROR +invalidation\.unsigned' <<<"$IV_OUT" | grep -q 'never counts' \
    && iv_clean && iv_no_disputes; then
    iv_ok "an unsigned invalidation: refused" "invalidation.unsigned, though its signer is eligible; nothing written"
else
    iv_fail "an unsigned invalidation: refused" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi
# The signing path offers the act to neither: who may sign is a human
# holding resolver who is not the performer.
IV_OUT=$(iv_war sign "$IV_G" --grounds "$IV_GROUNDS" --as claude --dry-run 2>&1); IV_S1=$?
IV_OUT2=$(iv_war sign "$IV_G" --grounds "$IV_GROUNDS" --as plant-agent --dry-run 2>&1); IV_S2=$?
if [[ $IV_S1 -ne 0 && $IV_S2 -ne 0 ]] && grep -E '^ERROR +sign\.who' <<<"$IV_OUT" | grep -q 'permits: Plant Signer' \
    && grep -qE '^ERROR +sign\.who' <<<"$IV_OUT2" && iv_clean; then
    iv_ok "war sign offers neither" "sign.who for the performer and the agent; only Plant Signer is eligible"
else
    iv_fail "war sign offers neither" "exit $IV_S1/$IV_S2: $(iv_lines "$IV_OUT") $(iv_lines "$IV_OUT2")"
fi
# The accepting dry run: every refusal passed, the signature the only thing
# missing, nothing written.
IV_OUT=$(iv_war sign "$IV_G" --grounds "$IV_GROUNDS" --as "Plant Signer" --dry-run 2>&1); IV_S=$?
if [[ $IV_S -eq 0 ]] && grep -E '^PASS +invalidation\.would-record' <<<"$IV_OUT" | grep -qF "($IV_A, $IV_B, $IV_C)" \
    && grep -qE '^PASS +sign\.would-record' <<<"$IV_OUT" && iv_clean && iv_no_disputes \
    && [[ "$(iv_records)" == "$IV_BEFORE" ]]; then
    iv_ok "the resolver's dry run would record" "invalidation.would-record ($IV_A, $IV_B, $IV_C); nothing written"
else
    iv_fail "the resolver's dry run would record" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi

# The act, signed by the throwaway key in a throwaway agent asserted to hold
# only it, under -euo pipefail: a failed setup step never reaches a signature.
iv_history() { # every record the obligations say is never rewritten
    (cd "$PLANT_ROOT" && find docs/warrants docs/gates -type f \
        \( -name resolution.toml -o -path '*/gate-runs/*' -o -path 'docs/warrants/*/attestations/*' -o -path 'docs/gates/*.yaml' \) \
        -print0 | sort -z | xargs -0 sha256sum | sha256sum)
}
IV_HIST=$(iv_history)
IV_DEF=$(sha256sum < "$PLANT_ROOT/docs/gates/$IV_G.yaml")
if WAR="$REPO_ROOT/$WAR" D="$PLANT_ROOT" T="$IV_TMP" G="$IV_G" GROUNDS="$IV_GROUNDS" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$IV_TMP/sign.log" 2>&1 <<'SIGN'
cd "$D"
eval "$(ssh-agent -s)" >/dev/null
trap 'ssh-agent -k >/dev/null 2>&1 || true' EXIT
ssh-add -q "$T/id_plant"
keys=$(ssh-add -l)
fp=$(ssh-keygen -lf "$T/id_plant.pub" | awk '{print $2}')
[[ $(grep -c . <<<"$keys") -eq 1 ]] && grep -qF -- "$fp" <<<"$keys" \
    || { echo "the throwaway agent holds more than the plant key: $keys"; exit 1; }
"$WAR" --root . sign "$G" --grounds "$GROUNDS" --ssh-sign --as "Plant Signer" </dev/null
SIGN
then IV_S=0; else IV_S=$?; fi
IV_OUT=$(cat "$IV_TMP/sign.log")
if [[ $IV_S -eq 0 ]] && grep -qE '^PASS +invalidation\.recorded' <<<"$IV_OUT" \
    && grep -qE '^PASS +attest\.emitted' <<<"$IV_OUT" && [[ -f "$IV_INV" ]] \
    && [[ -f "$PLANT_ROOT/docs/authority/responses/$IV_G.invalidation.response.toml.sig" ]]; then
    iv_ok "a resolver signs the invalidation" "invalidation.recorded and attested; the response carries its .sig"
else
    iv_fail "a resolver signs the invalidation" "exit $IV_S: $(iv_lines "$IV_OUT") $(tail -2 <<<"$IV_OUT" | tr '\n' ' ')"
fi

# OBL-003: exactly three disputes — A and B through their receipts, C through
# its parent A — each with §56.4's six fields; D, the control, gets none.
if python3 - "$PLANT_ROOT" "$IV_A" "$IV_B" "$IV_C" <<'PY'
import glob, sys, tomllib
root, a, b, c = sys.argv[1:5]
found = sorted(glob.glob(f"{root}/docs/warrants/*/disputes/*.toml"))
want = {f"{root}/docs/warrants/{w}/disputes/DSP-001.toml" for w in (a, b, c)}
if set(found) != want:
    sys.exit(f"disputes {found}")
for w in (a, b, c):
    d = tomllib.load(open(f"{root}/docs/warrants/{w}/disputes/DSP-001.toml", "rb"))
    res = tomllib.load(open(f"{root}/docs/warrants/{w}/resolution.toml", "rb"))["resolution"]
    x = d["dispute"]
    six = ["challenged_resolution", "grounds", "affected_evidence_or_judgment",
           "reliance_policy", "owner", "required_re_verification"]
    if any(not x.get(k) for k in six):
        sys.exit(f"{w}: a §56.4 field is empty")
    if x["challenged_resolution"] != res["id"]:
        sys.exit(f"{w}: challenges {x['challenged_resolution']}")
    if x["owner"] != "person://Plant Signer":
        sys.exit(f"{w}: owner {x['owner']}")
    if d["status"] != "open" or d["warrant"] != w:
        sys.exit(f"{w}: {d['status']} {d['warrant']}")
cd = tomllib.load(open(f"{root}/docs/warrants/{c}/disputes/DSP-001.toml", "rb"))["dispute"]
if cd["affected_evidence_or_judgment"] != [f"the resolution of {a}"]:
    sys.exit("C is not reached through A")
PY
then
    iv_ok "three disputes, §56.4's six fields" "$IV_A, $IV_B direct; $IV_C through $IV_A; owner the signer; $IV_D none"
else
    iv_fail "three disputes, §56.4's six fields" "the disputes written are not A, B, C with six fields each"
fi
IV_OUT=$(iv_war check 2>&1)
IV_DISPUTED=$(grep -E '^WARN +resolution\.disputed' <<<"$IV_OUT" | grep -oE 'IV-WAR-000[0-9]' | sort -u | tr '\n' ' ')
if [[ "$IV_DISPUTED" == "$IV_A $IV_B $IV_C " ]] && ! grep -q '^ERROR' <<<"$IV_OUT"; then
    iv_ok "war check: resolution.disputed" "for $IV_A, $IV_B, $IV_C and not $IV_D; 0 errors"
else
    iv_fail "war check: resolution.disputed" "disputed: '$IV_DISPUTED'; $(grep '^ERROR' <<<"$IV_OUT" | head -2 | tr '\n' '|')"
fi

# OBL-004: nothing historical rewritten; the definition file untouched;
# every attestation, the invalidation's own included, verifies.
IV_OUT=$(iv_war attest --all --verify 2>&1); IV_S=$?
if [[ "$(iv_history)" == "$IV_HIST" && "$(sha256sum < "$PLANT_ROOT/docs/gates/$IV_G.yaml")" == "$IV_DEF" ]] \
    && [[ $IV_S -eq 0 ]] && ! grep -q '^ERROR' <<<"$IV_OUT" \
    && compgen -G "$PLANT_ROOT/docs/gates/invalidations/attestations/invalidate-$IV_G-*.dsse.json" >/dev/null; then
    iv_ok "nothing historical is rewritten" "resolutions, receipts, runs, attestations, G's definition byte-identical; attest --all verifies"
else
    iv_fail "nothing historical is rewritten" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi
IV_OUT=$(iv_war check "$IV_E" 2>&1)
IV_OUT2=$(iv_war resolve "$IV_E" --dry-run 2>&1); IV_S=$?
if grep -E '^WARN +evidence\.gate-invalidated' <<<"$IV_OUT" | grep -qF "docs/gates/invalidations/$IV_G.toml" \
    && ! grep -E '^PASS +evidence\.admissible' <<<"$IV_OUT" | grep -qF "$IV_G" && [[ $IV_S -ne 0 ]]; then
    iv_ok "G's receipt is inadmissible on E" "evidence.gate-invalidated names the invalidation; $IV_E no longer resolvable"
else
    iv_fail "G's receipt is inadmissible on E" "exit $IV_S: $(iv_lines "$IV_OUT") $(iv_lines "$IV_OUT2")"
fi
git -C "$PLANT_ROOT" add -A
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "invalidated $IV_G"

# Once only: a second invalidation of the same version is refused.
IV_OUT=$(iv_war sign "$IV_G" --grounds "again" --as "Plant Signer" --dry-run 2>&1); IV_S=$?
if [[ $IV_S -ne 0 ]] && grep -qE '^ERROR +invalidation\.exists' <<<"$IV_OUT" && iv_clean; then
    iv_ok "a second invalidation: refused" "invalidation.exists; nothing written"
else
    iv_fail "a second invalidation: refused" "exit $IV_S: $(iv_lines "$IV_OUT")"
fi
# An edited dispute stops counting, and says so.
printf '# edited after it was written\n' >> "$IV_W/$IV_B/disputes/DSP-001.toml"
IV_OUT=$(iv_war check "$IV_B" 2>&1)
if grep -qE '^ERROR +resolution\.dispute-edited' <<<"$IV_OUT" && ! grep -q 'resolution.disputed' <<<"$IV_OUT"; then
    iv_ok "an edited dispute: refused" "resolution.dispute-edited; not read as standing"
else
    iv_fail "an edited dispute: refused" "$(iv_lines "$IV_OUT")"
fi
git -C "$PLANT_ROOT" checkout -q -- "docs/warrants/$IV_B/disputes/DSP-001.toml"
# The signature is what makes it count: without its .sig the same records
# dispute nothing, and E's receipt counts again.
IV_SIG="$PLANT_ROOT/docs/authority/responses/$IV_G.invalidation.response.toml.sig"
mv "$IV_SIG" "$IV_TMP/held.sig"
IV_OUT=$(iv_war check 2>&1)
IV_OUT2=$(iv_war check "$IV_E" 2>&1)
if [[ $(grep -cE '^ERROR +resolution\.dispute-unsigned' <<<"$IV_OUT") -eq 3 ]] \
    && ! grep -q 'resolution.disputed' <<<"$IV_OUT" \
    && grep -qE '^WARN +evidence\.invalidation-not-counted' <<<"$IV_OUT2" \
    && grep -E '^PASS +evidence\.admissible' <<<"$IV_OUT2" | grep -qF "$IV_G"; then
    iv_ok "unsigned, it never counts" "3x resolution.dispute-unsigned, none disputed; E's receipt admissible"
else
    iv_fail "unsigned, it never counts" "$(iv_lines "$IV_OUT") $(iv_lines "$IV_OUT2")"
fi
mv "$IV_TMP/held.sig" "$IV_SIG"
if [[ $(iv_war check 2>&1 | grep -cE '^WARN +resolution\.disputed') -eq 3 ]] && iv_clean; then
    iv_ok "signed again, it counts again" "the .sig restored: three resolutions disputed"
else
    iv_fail "signed again, it counts again" "$(git -C "$PLANT_ROOT" status --porcelain | head -2 | tr '\n' ' ')"
fi

command rm -rf "$IV_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT IV_TMP IV_BEFORE IV_HIST IV_DEF IV_INV IV_SIG
