# shellcheck shell=bash
# OW-WAR-0135 — the assurance mark: its decision record and baseline v1.
#
# OBL-001's structural half only. Whether the baseline is the right one, and
# the ADR's acceptance, are the owner's (document.review), and no plant can
# observe them. What a plant can observe: the ADR is an ADR `war check`
# reads, and the baseline names, for every requirement, the record that
# evidences it and what UNKNOWN means for it, in agreement with the ADR.
#
# OBL-002..OBL-005 exercise `war mark` (the second half of this file).
# CONTINGENT: `war mark` is built against OW-ADR-0025's recommendation
# (Q-001 (a) with (c) as a cache; Q-002 as baseline-v1.toml proposes), and
# OW-WAR-0135's work order holds it until the owner accepts the ADR. While
# this repository's baseline says `proposed`, it earns no mark, and the first
# plant below shows exactly that. To show a mark being earned at all, the
# scratch program carries its OWN copy of the baseline with status
# `accepted`: a fixture of that throwaway program, not this repository's
# decision.

echo "== assurance mark: decision and baseline (OW-WAR-0135) =="
PLANT_ROOT=$(scratch_corpus AM)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
AM_ADR=docs/adr/atoms/OW-ADR-0025-assurance-mark.md
AM_BL=docs/assurance/baseline-v1.toml
[[ -f "$AM_ADR" && -f "$AM_BL" ]] || { printf 'PLANT SETUP FAILED: %s or %s missing\n' "$AM_ADR" "$AM_BL" >&2; exit 9; }
am_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
am_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# The baseline's shape, and its agreement with the ADR. Prints nothing when
# the pair holds; otherwise one line per defect.
am_baseline_defects() { # baseline file, ADR file
    python3 - "$1" "$2" <<'PY'
import re, sys, tomllib
bl_path, adr_path = sys.argv[1], sys.argv[2]
try:
    bl = tomllib.load(open(bl_path, "rb"))
except Exception as e:
    print(f"baseline does not parse: {e}"); sys.exit(0)
adr = open(adr_path).read()
for key in ("schema", "id", "status", "independence_floor"):
    if not str(bl.get(key, "")).strip():
        print(f"baseline has no {key}")
if bl.get("status") not in ("proposed", "accepted"):
    print(f"baseline status {bl.get('status')!r} is neither proposed nor accepted")
reqs = bl.get("requirement", [])
if not reqs:
    print("baseline has no requirement")
seen = set()
for r in reqs:
    rid = str(r.get("id", "")).strip() or "<no id>"
    if rid in seen:
        print(f"{rid} appears twice")
    seen.add(rid)
    for field in ("check", "statement", "evidence", "unmet", "unknown"):
        if not str(r.get(field, "")).strip():
            print(f"{rid} has no {field}")
    if not re.search(r"(?m)^\| " + re.escape(rid) + r" \|", adr):
        print(f"{rid} is not in the ADR's baseline table")
for rid in re.findall(r"(?m)^\| (BL-\d+) \|", adr):
    if rid not in seen:
        print(f"the ADR's {rid} is not in the baseline")
# Q-001: the answer or the pending state, and every option with it.
for opt in ("### (a)", "### (b)", "### (c)"):
    if opt not in adr:
        print(f"the ADR does not record Q-001 option {opt[4:]}")
if not re.search(r"(?m)^## Decision$", adr):
    print("the ADR has no Decision section")
PY
}

# OBL-001, accept: the ADR is read as an ADR, beside the scratch program's own.
mkdir -p "$PLANT_ROOT/docs/adr/atoms"
command cp "$AM_ADR" "$PLANT_ROOT/docs/adr/atoms/"
AM_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
if grep -q 'adr.parsed' <<<"$AM_CHECK" && ! grep -q 'adr.malformed' <<<"$AM_CHECK"; then
    am_ok "the mark ADR is an ADR" "$(grep -o '[0-9]* ADR(s) parsed' <<<"$AM_CHECK" | head -1)"
else
    am_fail "the mark ADR is an ADR" "$(grep -E 'adr\.' <<<"$AM_CHECK" | head -2)"
fi
# Refusal: the same file with a status no ADR may hold is malformed, by name.
sed -i 's/^status: proposed$/status: decided-by-its-author/' "$PLANT_ROOT/docs/adr/atoms/OW-ADR-0025-assurance-mark.md"
AM_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
if grep -q 'adr.malformed' <<<"$AM_CHECK" && grep -q 'OW-ADR-0025' <<<"$(grep -A1 'adr.malformed' <<<"$AM_CHECK")"; then
    am_ok "a mangled mark ADR is refused" "adr.malformed, at OW-ADR-0025"
else
    am_fail "a mangled mark ADR is refused" "$(grep -E 'adr\.' <<<"$AM_CHECK" | head -2)"
fi

# OBL-001, accept: every requirement names its evidence and its UNKNOWN, and
# the baseline and the ADR list the same requirements.
AM_DEFECTS=$(am_baseline_defects "$AM_BL" "$AM_ADR")
if [[ -z "$AM_DEFECTS" ]]; then
    am_ok "baseline v1 names its evidence" "$(grep -c '^\[\[requirement\]\]' "$AM_BL") requirement(s), each with evidence, unmet, unknown"
else
    am_fail "baseline v1 names its evidence" "$(head -1 <<<"$AM_DEFECTS")"
fi
# Refusal: a requirement stripped of its evidence, and one the ADR never
# decided, are each named. The control against a predicate that passes all.
AM_BAD="$PLANT_ROOT/baseline-bad.toml"
python3 - "$AM_BL" "$AM_BAD" <<'PY'
import re, sys
t = open(sys.argv[1]).read()
t = re.sub(r'(?m)^evidence = .*\n', '', t, count=1)
t += '\n[[requirement]]\nid = "BL-099"\ntitle = "undecided"\ncheck = "x"\nstatement = "x"\nevidence = "x"\nunmet = "x"\nunknown = "x"\n'
open(sys.argv[2], "w").write(t)
PY
AM_DEFECTS=$(am_baseline_defects "$AM_BAD" "$AM_ADR")
if grep -q '^BL-001 has no evidence$' <<<"$AM_DEFECTS" && grep -q '^BL-099 is not in the ADR' <<<"$AM_DEFECTS"; then
    am_ok "an unevidenced requirement is named" "BL-001 has no evidence; BL-099 not in the ADR"
else
    am_fail "an unevidenced requirement is named" "${AM_DEFECTS:-no defect reported}"
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT AM_CHECK AM_DEFECTS AM_BAD

# ═══ OBL-002..OBL-005: `war mark` over a resolved scratch Warrant ═══════════
#
# One Warrant resolved end to end — authorized, its gate run, independently
# verified, resolved — by a key generated here and held by a throwaway
# ssh-agent that holds nothing else (the pattern of 49 and 56). Nothing here is
# the owner's key or this repository's corpus. The Warrant cites
# `gate://plant.mark@1.0.0`, whose definition declares `inputs: ["src/**"]`, so
# `src/` is the candidate BL-005 reads and `README.txt` is not.
echo "== assurance mark: war mark (OW-WAR-0135 M2, contingent) =="
PLANT_ROOT=$(scratch_corpus MK)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
MK_TMP=$(mktemp -d)
MK_A="MK-WAR-0001"
MK_W="$PLANT_ROOT/docs/warrants/$MK_A"
MK_BL="$PLANT_ROOT/docs/assurance/baseline-v1.toml"

if ! WAR="$REPO_ROOT/$WAR" D="$PLANT_ROOT" T="$MK_TMP" A="$MK_A" BL="$REPO_ROOT/$AM_BL" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$MK_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
W="docs/warrants/$A"
mkdir -p src docs/assurance
printf 'accepted\n' > src/core.txt
printf 'helper\n' > src/helper.txt
printf 'notes\n' > README.txt
# This repository's baseline, byte for byte: `proposed`, as it is here.
cp "$BL" docs/assurance/baseline-v1.toml
cat > docs/gates/plant.mark@1.0.0.yaml <<'GATE'
gate_id: "plant.mark"
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
known_blind_spots: ["Passes whenever true exits 0; it stands in for a gate that reads src/."]
qualification_qualifier: "conformance/plants.d/57-assurance-mark.sh"
qualification_digest: ""
qualification_positive_controls: ["The plant's Warrant resolves with it."]
qualification_negative_controls: ["None; it qualifies the seam, not any work."]
qualification_mutation_classes: ["none"]
qualification_environments: ["linux-x86_64"]
qualification_limitations: ["A gate that always passes qualifies nothing but its own execution."]
detection_results:
  - fault_class: "plant-cannot-run"
    mutation: "none"
    detected: "true"
GATE
python3 - "$W/atoms/60-assurance.md" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.split("## Acceptance Obligations")[0] + """## Acceptance Obligations

### OBL-001 — the scratch file says accepted
- **scope:** `src/core.txt` in this scratch program.
- **gate:** `gate://plant.mark@1.0.0`
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
sed -i 's/"OBL-002"/"OBL-001"/' "$W/atoms/45-milestones.yaml"
grep -q '"OBL-001"' "$W/atoms/45-milestones.yaml"
cat > "$W/deliverables.toml" <<DELIV
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "the accepted file"
kind = "file"
target_ref = "src/core.txt"
required = true
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001"]

[deliverable.provenance]
producer = "claude"
producing_attempt = "$A/attempt-1"
contract_digest = "unrecorded"
tool_or_runtime_identity = "plant"
creation_method = "authored"
content_digest = "sha256:$(sha256sum src/core.txt | cut -d' ' -f1)"
media_type = "text/plain"
classification = "internal"
retention = "repository-lifetime"
source_holder = "git"
DELIV
printf 'schema = "oh.war/rationale/v1"\n' > "$W/rationale.toml"
ssh-keygen -q -t ed25519 -N "" -C plant -f "$T/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$T/id_plant.pub")" > docs/authority/allowed_signers
cat > docs/authority/roles.toml <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/57-assurance-mark.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the assurance-mark plants run."
ssh_principal = "plant"
ROLES
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "a Warrant to mark"

eval "$(ssh-agent -s)" >/dev/null
trap 'ssh-agent -k >/dev/null 2>&1 || true' EXIT
ssh-add -q "$T/id_plant"
# The agent must hold this key and nothing else: a signature by any other
# key would be a signature this plant has no business making.
keys=$(ssh-add -l)
fp=$(ssh-keygen -lf "$T/id_plant.pub" | awk '{print $2}')
[[ $(grep -c . <<<"$keys") -eq 1 ]] && grep -qF -- "$fp" <<<"$keys" \
    || { echo "the throwaway agent holds more than the plant key: $keys"; exit 1; }

"$WAR" --root . sign "$A" --ssh-sign --as "Plant Signer" </dev/null
g add -A; g commit -qm authorized
"$WAR" --root . evidence record "$A"
g add -A; g commit -qm evidence
cat > "$T/verified.toml" <<VERIFY
schema = "oh.war/verification-response/v1"
warrant = "$A"

[[verifications]]
obligation = "OBL-001"
disposition = "established"
evidence = "src/core.txt read at the accepted candidate"
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
"$WAR" --root . verify "$A" --response "$T/verified.toml"
g add -A; g commit -qm verified
"$WAR" --root . sign "$A" --ssh-sign --as "Plant Signer" </dev/null
test -f "$W/resolution.toml"
grep -q '^commit_sha = ' "$W/resolution.toml"
ls "$W"/attestations/resolve-*.dsse.json >/dev/null
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm resolved
SETUP
then
    printf 'PLANT SETUP FAILED: could not resolve a scratch Warrant:\n' >&2
    tail -15 "$MK_TMP/setup.log" >&2
    exit 9
fi

mk_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
mk_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
mk_war()  { "$WAR" --root "$PLANT_ROOT" "$@"; }
mk_commit() { git -C "$PLANT_ROOT" add -A >/dev/null 2>&1 && git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1; }
mk_lines() { grep -E '^(PASS|ERROR|UNKNOWN|WARN)' <<<"$1" | grep -v 'requirement-met' | head -4 | tr -s ' ' | tr '\n' '|'; }
# One requirement's result in the --json evaluation: met | unmet | unknown.
mk_result() { # json, requirement id
    python3 -c 'import json,sys; r=json.loads(sys.stdin.read())["result"]["requirements"]; print(next((x["result"] for x in r if x["id"]==sys.argv[1]), "absent"))' "$2" <<<"$1" 2>/dev/null
}
# The detail of one requirement in the --json evaluation.
mk_detail() { # json, requirement id
    python3 -c 'import json,sys; r=json.loads(sys.stdin.read())["result"]["requirements"]; print(next((x["detail"] for x in r if x["id"]==sys.argv[1]), ""))' "$2" <<<"$1" 2>/dev/null
}
mk_has_mark() { python3 -c 'import json,sys; sys.exit(0 if json.loads(sys.stdin.read())["result"].get("mark") else 1)' <<<"$1" 2>/dev/null; }
# Back to the last commit: tracked files restored, nothing new left behind.
mk_restore() { git -C "$PLANT_ROOT" checkout -q -- . && git -C "$PLANT_ROOT" clean -qfd >/dev/null 2>&1; }

# ── This repository's state: a proposed baseline earns no mark ──────────────
# The scratch Warrant meets every requirement; the baseline is this
# repository's own file, `proposed`. The refusal names the baseline, and it
# is the ONLY thing that stands between the Warrant and a mark.
MK_OUT=$(mk_war mark "$MK_A" --json 2>&1); MK_STATUS=$?
MK_MET=$(python3 -c 'import json,sys; r=json.loads(sys.stdin.read())["result"]["requirements"]; print(sum(x["result"]=="met" for x in r), len(r))' <<<"$MK_OUT" 2>/dev/null)
if [[ $MK_STATUS -ne 0 ]] && grep -q '"rule": "mark.baseline-not-accepted"' <<<"$MK_OUT" \
    && ! grep -q '"rule": "mark.earned"' <<<"$MK_OUT" && ! mk_has_mark "$MK_OUT" && [[ "$MK_MET" == "6 6" ]]; then
    mk_ok "a proposed baseline earns no mark" "mark.baseline-not-accepted; 6/6 requirements met, no statement (exit $MK_STATUS)"
else
    mk_fail "a proposed baseline earns no mark" "exit $MK_STATUS, met $MK_MET: $(mk_lines "$(mk_war mark "$MK_A" 2>&1)")"
fi
# OBL-002 refusal: nothing is written when nothing was earned.
MK_OUT=$(mk_war mark "$MK_A" --record 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -ne 0 ]] && grep -q 'mark.not-recorded' <<<"$MK_OUT" && [[ ! -e "$MK_W/mark-v1.json" ]]; then
    mk_ok "no mark, nothing recorded" "mark.not-recorded; no mark-v1.json (exit $MK_STATUS)"
else
    mk_fail "no mark, nothing recorded" "exit $MK_STATUS: $(mk_lines "$MK_OUT")"
fi
# Without a file, v1 is the one this build ships, and it is proposed too.
command mv "$MK_BL" "$MK_TMP/baseline-v1.toml"
MK_OUT=$(mk_war mark "$MK_A" 2>&1); MK_STATUS=$?
command mv "$MK_TMP/baseline-v1.toml" "$MK_BL"
if [[ $MK_STATUS -ne 0 ]] && grep -q 'mark.baseline-not-accepted' <<<"$MK_OUT" && ! grep -q 'mark.earned' <<<"$MK_OUT"; then
    mk_ok "the built-in v1 is proposed" "mark.baseline-not-accepted with no baseline file (exit $MK_STATUS)"
else
    mk_fail "the built-in v1 is proposed" "exit $MK_STATUS: $(mk_lines "$MK_OUT")"
fi

# The scratch program's own fixture: its copy says accepted from here on.
sed -i 's/^status = "proposed"$/status = "accepted"/' "$MK_BL"
assert_present 'status = "accepted"' "$MK_BL"
mk_commit "the scratch program accepts its copy of the baseline (a fixture)"

# ── OBL-002: a Warrant that meets the baseline gets a mark naming what it binds
MK_OUT=$(mk_war mark "$MK_A" --json 2>&1); MK_STATUS=$?
printf '%s' "$MK_OUT" > "$MK_TMP/earned.json"
MK_CHECK=$(python3 - "$PLANT_ROOT" "$MK_A" "$MK_TMP/earned.json" 2>&1 <<'PY'
import hashlib, json, sys, tomllib, pathlib
root, alias = pathlib.Path(sys.argv[1]), sys.argv[2]
env = json.loads(open(sys.argv[3]).read())
m = env["result"].get("mark")
if not m:
    print("no mark in the result"); sys.exit(0)
w = root / "docs/warrants" / alias
sha = lambda p: "sha256:" + hashlib.sha256((root / p).read_bytes()).hexdigest()
res = tomllib.loads((w / "resolution.toml").read_text())
att = sorted(w.glob("attestations/resolve-*.dsse.json"))[-1].relative_to(root).as_posix()
want = {
    "schema": "oh.war/mark/v1",
    "warrant.alias": alias,
    "baseline.id": "v1",
    "baseline.sha256": sha("docs/assurance/baseline-v1.toml"),
    "baseline.independence_floor": "controlled",
    "resolution.sha256": sha(f"docs/warrants/{alias}/resolution.toml"),
    "attestation.path": att,
    "attestation.sha256": sha(att),
    "commit": res["locator"]["commit_sha"],
    "contract_digest": res["resolution"]["contract_digest"],
    "obligations": [["OBL-001", sha(f"docs/warrants/{alias}/verifications/OBL-001.toml")]],
    "requirements": [f"BL-00{i}" for i in range(1, 7)],
}
got = {
    "schema": m["schema"],
    "warrant.alias": m["warrant"]["alias"],
    "baseline.id": m["baseline"]["id"],
    "baseline.sha256": m["baseline"]["sha256"],
    "baseline.independence_floor": m["baseline"]["independence_floor"],
    "resolution.sha256": m["resolution"]["sha256"],
    "attestation.path": m["attestation"]["path"],
    "attestation.sha256": m["attestation"]["sha256"],
    "commit": m["commit"],
    "contract_digest": m["contract_digest"],
    "obligations": [[o["id"], o["sha256"]] for o in m["obligations"]],
    "requirements": [r["id"] for r in m["requirements"] if r["result"] == "met"],
}
for k in want:
    if want[k] != got[k]:
        print(f"{k}: mark says {got[k]!r}, the corpus recomputes {want[k]!r}")
if not m["warrant"]["uuid"]:
    print("warrant.uuid is empty")
PY
)
if [[ $MK_STATUS -eq 0 ]] && grep -q '"rule": "mark.earned"' <<<"$MK_OUT" && [[ -z "$MK_CHECK" ]]; then
    mk_ok "a met baseline earns a mark" "oh.war/mark/v1; baseline, resolution, attestation, commit, contract, OBL-001 each recompute"
else
    mk_fail "a met baseline earns a mark" "exit $MK_STATUS: ${MK_CHECK:-$(mk_lines "$(mk_war mark "$MK_A" 2>&1)")}"
fi
mk_statement() { mk_war mark "$MK_A" --json 2>/dev/null | python3 -c 'import json,sys; print(json.dumps(json.load(sys.stdin)["result"]["mark"], sort_keys=True))' 2>/dev/null; }
MK_ONE=$(mk_statement); MK_TWO=$(mk_statement)
if [[ -n "$MK_ONE" && "$MK_ONE" == "$MK_TWO" ]] && ! grep -qiE '"(recorded_at|generated_at|timestamp|war_version)"' <<<"$MK_ONE"; then
    mk_ok "a mark recomputes to the same bytes" "two evaluations, one statement; no timestamp, no build"
else
    mk_fail "a mark recomputes to the same bytes" "the two statements differ, or one carries a time"
fi

# ── OBL-003: no mark without a human-signed resolution ──────────────────────
# Accept: the resolution above is human-signed and attested, and says so.
MK_OUT=$(mk_war mark "$MK_A" --json 2>&1)
if [[ "$(mk_result "$MK_OUT" BL-001)/$(mk_result "$MK_OUT" BL-002)/$(mk_result "$MK_OUT" BL-003)" == "met/met/met" ]] \
    && grep -qF 'signed by Plant Signer (human)' <<<"$(mk_detail "$MK_OUT" BL-002)"; then
    mk_ok "a signed, attested resolution" "BL-001, BL-002 (Plant Signer, human), BL-003 met"
else
    mk_fail "a signed, attested resolution" "$(mk_result "$MK_OUT" BL-001)/$(mk_result "$MK_OUT" BL-002)/$(mk_result "$MK_OUT" BL-003)"
fi
# mk_refused <label> <requirement> <text the requirement's detail must hold>
mk_refused() {
    local out status
    out=$(mk_war mark "$MK_A" --json 2>&1); status=$?
    if [[ $status -ne 0 ]] && ! mk_has_mark "$out" && ! grep -q '"rule": "mark.earned"' <<<"$out" \
        && grep -q '"rule": "mark.refused"' <<<"$out" \
        && [[ "$(mk_result "$out" "$2")" == "unmet" ]] && grep -qF -- "$3" <<<"$(mk_detail "$out" "$2")"; then
        mk_ok "$1" "no mark; $2 unmet: $3 (exit $status)"
    else
        mk_fail "$1" "exit $status, $2 $(mk_result "$out" "$2"): $(mk_detail "$out" "$2" | head -c 300)"
    fi
}
# 1. No resolution: implementation finished, unreviewed (Q47).
command mv "$MK_W/resolution.toml" "$MK_TMP/resolution.toml"
assert_gone_file "$MK_W/resolution.toml"
mk_refused "no resolution, no mark" BL-001 "Q47"
command mv "$MK_TMP/resolution.toml" "$MK_W/resolution.toml"
# 2. A resolution with no attestation: what a TTY signature leaves.
command mv "$MK_W/attestations" "$MK_TMP/attestations"
assert_gone_file "$MK_W/attestations"
mk_refused "no attestation, no mark" BL-002 "no resolve attestation"
command mv "$MK_TMP/attestations" "$MK_W/attestations"
# 3. An attestation whose signature does not verify.
MK_ATT=$(ls "$MK_W"/attestations/resolve-*.dsse.json | tail -1)
python3 - "$MK_ATT" <<'PY'
import base64, json, sys
p = sys.argv[1]; e = json.load(open(p))
sig = bytearray(base64.b64decode(e["signatures"][0]["sig"]))
sig[-1] ^= 0x01
e["signatures"][0]["sig"] = base64.b64encode(bytes(sig)).decode()
json.dump(e, open(p, "w"))
PY
if git -C "$PLANT_ROOT" diff --quiet -- "$MK_ATT"; then printf 'PLANT MUTATION WAS A NO-OP: %s unchanged\n' "$MK_ATT" >&2; exit 9; fi
mk_refused "a forged attestation, no mark" BL-002 "attest.failed"
mk_restore
# 4. A resolution resting on a performer-only verdict (RQ-053).
sed -i 's/^actor = "plant-verifier"$/actor = "claude"/' "$MK_W/verifications/OBL-001.toml"
assert_present 'actor = "claude"' "$MK_W/verifications/OBL-001.toml"
mk_refused "a self-verified obligation, no mark" BL-003 "OBL-001: not admissible"
mk_restore
# A resolver the register does not hold as human: no delegated act earns it.
sed -i 's/^actor_kind = "human"$/actor_kind = "agent"/' "$PLANT_ROOT/docs/authority/roles.toml"
assert_present 'actor_kind = "agent"' "$PLANT_ROOT/docs/authority/roles.toml"
mk_refused "a non-human resolver, no mark" BL-002 "is not a human"
mk_restore

# ── OBL-004: UNKNOWN is not met, and a weaker baseline is not the same mark ──
# Accept: an extension that ADDS a requirement; the mark names both.
mk_extra() { # the check RX-001 names
    cat > "$PLANT_ROOT/docs/assurance/plant-extra.toml" <<EXTRA
schema = "oh.war/assurance-baseline/v1"
id = "plant-extra"
extends = "v1"
${2:-}

[[requirement]]
id = "RX-001"
title = "Every deliverable is content-addressed"
check = "$1"
EXTRA
}
mk_extra deliverables.content_addressed
printf '\n[mark]\nbaseline = "v1"\nextra = "docs/assurance/plant-extra.toml"\n' >> "$PLANT_ROOT/openwarrant.toml"
MK_OUT=$(mk_war mark "$MK_A" --json 2>&1); MK_STATUS=$?
MK_NAMES=$(python3 -c 'import json,sys; m=json.loads(sys.stdin.read())["result"]["mark"]; print(m["baseline"]["id"], ",".join(e["id"] for e in m["baseline"]["extensions"]), ",".join(r["id"] for r in m["requirements"]))' <<<"$MK_OUT" 2>/dev/null)
if [[ $MK_STATUS -eq 0 ]] && [[ "$MK_NAMES" == "v1 plant-extra BL-001,BL-002,BL-003,BL-004,BL-005,BL-006,RX-001" ]]; then
    mk_ok "a strengthened baseline names both" "v1 + plant-extra; RX-001 beside BL-001..BL-006"
else
    mk_fail "a strengthened baseline names both" "exit $MK_STATUS: ${MK_NAMES:-no mark} $(mk_lines "$(mk_war mark "$MK_A" 2>&1)")"
fi
# Refusal: an extension requirement no check can answer is UNKNOWN, not met.
mk_extra fixtures.before_implementation
MK_OUT=$(mk_war mark "$MK_A" --json 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -ne 0 ]] && ! mk_has_mark "$MK_OUT" && [[ "$(mk_result "$MK_OUT" RX-001)" == "unknown" ]] \
    && grep -q '"rule": "mark.requirement-unknown"' <<<"$MK_OUT" && ! grep -q '"rule": "mark.requirement-unmet"' <<<"$MK_OUT"; then
    mk_ok "an unanswerable requirement" "RX-001 UNKNOWN (no such check), no mark, nothing unmet"
else
    mk_fail "an unanswerable requirement" "exit $MK_STATUS, RX-001 $(mk_result "$MK_OUT" RX-001)"
fi
# Refusal: an extension that lowers the floor is weaker, and refused by name.
mk_extra deliverables.content_addressed 'independence_floor = "basic"'
MK_OUT=$(mk_war mark "$MK_A" 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -ne 0 ]] && grep -qE 'mark\.baseline-weakened +weakened, so not v1: .*lowers independence_floor to basic' <<<"$MK_OUT" && ! grep -q 'mark.earned' <<<"$MK_OUT"; then
    mk_ok "an extension cannot lower the floor" "mark.baseline-weakened: independence_floor to basic"
else
    mk_fail "an extension cannot lower the floor" "exit $MK_STATUS: $(mk_lines "$MK_OUT")"
fi
mk_restore
# Refusal: a repository copy of v1 with BL-004 removed is not v1.
python3 - "$MK_BL" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p).read()
s = re.sub(r'\[\[requirement\]\]\nid = "BL-004"\n(?:(?!\[\[).*\n?)*', '', s)
open(p, "w").write(s)
PY
assert_gone 'id = "BL-004"' "$MK_BL"
MK_OUT=$(mk_war mark "$MK_A" 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -ne 0 ]] && grep -qE 'mark\.baseline-weakened +weakened, so not v1: BL-004 removed' <<<"$MK_OUT" && ! grep -q 'mark.earned' <<<"$MK_OUT"; then
    mk_ok "a v1 with BL-004 removed" "mark.baseline-weakened: BL-004 removed; no v1 mark"
else
    mk_fail "a v1 with BL-004 removed" "exit $MK_STATUS: $(mk_lines "$MK_OUT")"
fi
mk_restore
# Refusal: a resolution with no locator — BL-004 is UNKNOWN, never met.
python3 - "$MK_W/resolution.toml" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p).read()
open(p, "w").write(re.sub(r'\n\[locator\]\n(?:(?!\[).*\n?)*', '\n', s))
PY
assert_gone '[locator]' "$MK_W/resolution.toml"
MK_OUT=$(mk_war mark "$MK_A" --json 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -ne 0 ]] && ! mk_has_mark "$MK_OUT" && [[ "$(mk_result "$MK_OUT" BL-004)" == "unknown" ]] \
    && grep -qF 'no [locator]' <<<"$(mk_detail "$MK_OUT" BL-004)" && grep -q '"rule": "mark.requirement-unknown"' <<<"$MK_OUT"; then
    mk_ok "no locator is UNKNOWN, not met" "BL-004 UNKNOWN (no [locator]); no mark"
else
    mk_fail "no locator is UNKNOWN, not met" "exit $MK_STATUS, BL-004 $(mk_result "$MK_OUT" BL-004)"
fi
mk_restore

# ── OBL-005: a mark goes stale when what it binds moves ─────────────────────
MK_OUT=$(mk_war mark "$MK_A" --record 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -eq 0 ]] && grep -q 'mark.recorded' <<<"$MK_OUT" && [[ -f "$MK_W/mark-v1.json" ]]; then
    mk_ok "an earned mark is recorded" "mark-v1.json written"
else
    mk_fail "an earned mark is recorded" "exit $MK_STATUS: $(mk_lines "$MK_OUT")"
fi
mk_commit "the mark, recorded"
MK_CHECK=$(mk_war check 2>&1)
if ! grep -qE '^(ERROR|UNKNOWN)' <<<"$MK_CHECK"; then
    mk_ok "a recorded mark leaves check clean" "war check: no error, no unknown"
else
    mk_fail "a recorded mark leaves check clean" "$(mk_lines "$MK_CHECK")"
fi
# Accept, and the control against a --verify that always fails.
MK_OUT=$(mk_war mark "$MK_A" --verify 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -eq 0 ]] && grep -q 'mark.verified' <<<"$MK_OUT" && ! grep -q 'mark.stale' <<<"$MK_OUT"; then
    mk_ok "an unchanged mark verifies" "mark.verified (exit 0)"
else
    mk_fail "an unchanged mark verifies" "exit $MK_STATUS: $(mk_lines "$MK_OUT")"
fi
# mk_stale <label> <binding the finding must name>
mk_stale() {
    local out status
    out=$(mk_war mark "$MK_A" --verify 2>&1); status=$?
    if [[ $status -ne 0 ]] && grep -qE "mark\.stale +$MK_A: \`$2\` moved" <<<"$out" && ! grep -q 'mark.verified' <<<"$out"; then
        mk_ok "$1" "mark.stale names \`$2\` (exit $status)"
    else
        mk_fail "$1" "exit $status: $(mk_lines "$out")"
    fi
}
# The resolution file edited after the mark.
printf '# edited after the mark\n' >> "$MK_W/resolution.toml"
mk_stale "an edited resolution" resolution
mk_restore
# An in-scope source change, committed: the candidate moved (OW-WAR-0134).
MK_HEAD=$(git -C "$PLANT_ROOT" rev-parse HEAD)
printf 'changed after the mark\n' >> "$PLANT_ROOT/src/helper.txt"
mk_commit "an in-scope change"
mk_stale "an in-scope change" candidate
git -C "$PLANT_ROOT" reset -q --hard "$MK_HEAD"
# An out-of-scope change moves nothing the mark binds.
printf 'more notes\n' >> "$PLANT_ROOT/README.txt"
mk_commit "an unrelated change"
MK_OUT=$(mk_war mark "$MK_A" --verify 2>&1); MK_STATUS=$?
if [[ $MK_STATUS -eq 0 ]] && grep -q 'mark.verified' <<<"$MK_OUT"; then
    mk_ok "an out-of-scope change" "still mark.verified (README.txt is not the candidate)"
else
    mk_fail "an out-of-scope change" "exit $MK_STATUS: $(mk_lines "$MK_OUT")"
fi
git -C "$PLANT_ROOT" reset -q --hard "$MK_HEAD"
# A hand-edited mark is a claim to check, not a mark.
sed -i 's/"commit": "\([0-9a-f]\{39\}\)[0-9a-f]"/"commit": "\1x"/' "$MK_W/mark-v1.json"
assert_present '"commit": "' "$MK_W/mark-v1.json"
if git -C "$PLANT_ROOT" diff --quiet -- "docs/warrants/$MK_A/mark-v1.json"; then printf 'PLANT MUTATION WAS A NO-OP: mark-v1.json commit unchanged\n' >&2; exit 9; fi
mk_stale "a hand-edited mark" commit
mk_restore

command rm -rf "$MK_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT MK_TMP MK_A MK_W MK_BL MK_OUT MK_STATUS MK_CHECK MK_MET MK_NAMES MK_ONE MK_TWO MK_ATT MK_HEAD AM_ADR AM_BL
