# shellcheck shell=bash
# OW-WAR-0134 — acceptance stays valid only for the candidate accepted.
#
# A scratch program with one Warrant resolved end to end — authorized, its gate
# run, independently verified, resolved — by a key generated here and held by
# a throwaway ssh-agent that holds nothing else. Nothing here is the owner's
# key or this repository's corpus.
#
# The Warrant cites `gate://plant.acceptance@1.0.0`, whose definition declares
# `inputs: ["src/**"]` (OW-WAR-0133 Q-001 (c)), so `src/` is in scope and
# `README.txt` is not. Its one pinned deliverable is `src/core.txt`.
#
# Every claim is paired with the control seen refusing, and every finding is
# matched by the rule that fired, not by an exit code.

echo "== acceptance validity (OW-WAR-0134) =="
PLANT_ROOT=$(scratch_corpus AV)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
AV_TMP=$(mktemp -d)
AV_A="AV-WAR-0001"
AV_RES="$PLANT_ROOT/docs/warrants/$AV_A/resolution.toml"

# ── Setup: a resolved Warrant, built under -euo pipefail ─────────────────────
# Any step that fails aborts the setup, so no plant below ever scores a
# half-built corpus. The agent lives only as long as the setup: nothing after
# it signs anything.
if ! WAR="$REPO_ROOT/$WAR" D="$PLANT_ROOT" T="$AV_TMP" A="$AV_A" \
    BIND="$REPO_ROOT/conformance/fixtures/verifier/with-subject.py" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$AV_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
W="docs/warrants/$A"
mkdir -p src
printf 'accepted\n' > src/core.txt
printf 'helper\n' > src/helper.txt
mkdir -p support
printf 'linked API context\n' > support/api.txt
ln -s ../support src/linked
printf 'notes\n' > README.txt
cat > docs/gates/plant.acceptance@1.0.0.yaml <<'GATE'
gate_id: "plant.acceptance"
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
qualification_qualifier: "conformance/plants.d/56-acceptance-validity.sh"
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
- **gate:** `gate://plant.acceptance@1.0.0`
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
assigned_by = "conformance/plants.d/56-acceptance-validity.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the acceptance-validity plants run."
ssh_principal = "plant"
ROLES
# Real manifests bind ADR atoms stored elsewhere inside the same repository.
mkdir -p docs/adr/atoms
printf '# Bound scratch decision\n\nThe scratch contract uses this decision.\n' > docs/adr/atoms/bound.md
cat >> "$W/manifest.toml" <<'BOUND'

[[atoms]]
ordinal = 35
role = "adr"
path = "../../adr/atoms/bound.md"
required = true
BOUND
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "a Warrant to accept"

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
g rev-parse HEAD > "$T/authorized.sha"
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
# Capture exactly the subject this synthetic fixture is about to inspect.
"$WAR" --root . verify "$A" --performer claude --bundle --json > "$T/request.json"
python3 "$BIND" "$T/request.json" "$T/verified.toml" > "$T/verified-bound.toml"
"$WAR" --root . verify "$A" --response "$T/verified-bound.toml"
g add -A; g commit -qm verified
"$WAR" --root . sign "$A" --ssh-sign --as "Plant Signer" </dev/null
test -f "$W/resolution.toml"
grep -q '^commit_sha = ' "$W/resolution.toml"
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm resolved
g rev-parse HEAD > "$T/resolved.sha"
SETUP
then
    printf 'PLANT SETUP FAILED: could not resolve a scratch Warrant:\n' >&2
    tail -15 "$AV_TMP/setup.log" >&2
    exit 9
fi
AV_RESOLVED=$(cat "$AV_TMP/resolved.sha")
AV_AUTHORIZED=$(cat "$AV_TMP/authorized.sha")

av_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
av_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
av_war()  { "$WAR" --root "$PLANT_ROOT" "$@"; }
av_commit() { git -C "$PLANT_ROOT" add -A >/dev/null 2>&1 && git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1; }
av_sha()  { sha256sum < "$AV_RES" | cut -d' ' -f1; }
av_lines() { grep -E '^(PASS|ERROR|UNKNOWN|WARN)' <<<"$1" | head -3 | tr '\n' '|'; }
# A finding naming a path: the rule and the path on the same diagnostic.
av_names() { line_has -F "$1" -F "$3" <<<"$2"; }
av_verify() { # disposition, evidence
    sed -e "s/^disposition = .*/disposition = \"$1\"/" -e "s|^evidence = .*|evidence = \"$2\"|" \
        "$AV_TMP/verified.toml" > "$AV_TMP/reverify.toml"
    av_war verify "$AV_A" --performer claude --bundle --json > "$AV_TMP/request.json"
    python3 "$REPO_ROOT/conformance/fixtures/verifier/with-subject.py" \
        "$AV_TMP/request.json" "$AV_TMP/reverify.toml" > "$AV_TMP/reverify-bound.toml"
    av_war verify "$AV_A" --response "$AV_TMP/reverify-bound.toml" >/dev/null 2>&1
}

# ── Control: the resolution commit itself is not a move ─────────────────────
AV_OUT=$(av_war pins --candidate 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -eq 0 ]] && grep -qE "PASS +acceptance\.unchanged +$AV_A" <<<"$AV_OUT"; then
    av_ok "the accepted candidate is unchanged" "acceptance.unchanged (exit 0)"
else
    av_fail "the accepted candidate is unchanged" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi

# The shared ADR link is allowed, but traversal beyond the candidate tree is not.
AV_ACCEPTED=$(git -C "$PLANT_ROOT" rev-parse HEAD)
sed -i 's|../../adr/atoms/bound.md|../../../../outside-candidate.md|' "$PLANT_ROOT/docs/warrants/$AV_A/manifest.toml"
av_commit "an atom reference outside the candidate tree"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && grep -q 'acceptance.unknown' <<<"$AV_OUT" && grep -q 'atom path escapes' <<<"$AV_OUT" && ! grep -q 'acceptance.unchanged' <<<"$AV_OUT"; then
    av_ok "escaping candidate atom is refused" "acceptance.unknown; no host file is read"
else
    av_fail "escaping candidate atom is refused" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
git -C "$PLANT_ROOT" reset -q --hard "$AV_ACCEPTED"

# ── OBL-002: an out-of-scope change leaves acceptance standing ──────────────
printf 'more notes\n' >> "$PLANT_ROOT/README.txt"
av_commit "an unrelated change"
AV_OUT=$(av_war pins --candidate HEAD --json 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -eq 0 ]] && grep -q '"rule": "acceptance.unchanged"' <<<"$AV_OUT" \
    && python3 -c 'import json,sys; a=json.loads(sys.stdin.read())["result"]["acceptance"]["warrants"][0]; sys.exit(0 if a["state"]=="unchanged" and a["out_of_scope"]==["README.txt"] and not a.get("in_scope") else 1)' <<<"$AV_OUT"; then
    av_ok "an out-of-scope change" "unchanged; README.txt out of scope (exit 0)"
else
    av_fail "an out-of-scope change" "exit $AV_STATUS: $(python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["result"]["acceptance"]["warrants"])' <<<"$AV_OUT" 2>&1 | head -c 600)"
fi

# ── OBL-001: an in-scope change that no deliverable pins is named ───────────
AV_BEFORE=$(av_sha)
printf 'changed after acceptance\n' >> "$PLANT_ROOT/src/helper.txt"
av_commit "an in-scope change"
AV_MOVED=$(git -C "$PLANT_ROOT" rev-parse HEAD)
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/helper.txt' \
    && ! grep -q 'acceptance.unknown' <<<"$AV_OUT" && ! grep -q 'README.txt' <<<"$(grep 'acceptance.candidate-moved' <<<"$AV_OUT")"; then
    av_ok "an in-scope change is named" "acceptance.candidate-moved: src/helper.txt (exit $AV_STATUS)"
else
    av_fail "an in-scope change is named" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
if [[ "$(av_sha)" == "$AV_BEFORE" ]] && git -C "$PLANT_ROOT" diff --quiet -- "docs/warrants/$AV_A/resolution.toml"; then
    av_ok "the resolution is untouched" "resolution.toml byte-identical"
else
    av_fail "the resolution is untouched" "resolution.toml changed"
fi
AV_OUT=$(av_war pins --candidate HEAD --json 2>&1)
if grep -q '"rule": "acceptance.candidate-moved"' <<<"$AV_OUT" && grep -q '"acceptance": "moved"' <<<"$AV_OUT"; then
    av_ok "the finding is in --json too" "rule acceptance.candidate-moved beside the pin"
else
    av_fail "the finding is in --json too" "$(grep -E '"rule"' <<<"$AV_OUT" | head -2 | tr -s ' \n' ' ')"
fi

# CI's question: with the target branch as base, an acceptance the branch
# carries is judged, and one already on the base has landed.
AV_OUT=$(av_war pins --candidate HEAD --base "$AV_AUTHORIZED" 2>&1); AV_STATUS=$?
AV_OUT2=$(av_war pins --candidate HEAD --base "$AV_RESOLVED" 2>&1); AV_STATUS2=$?
if [[ $AV_STATUS -ne 0 ]] && grep -q 'acceptance.candidate-moved' <<<"$AV_OUT" \
    && [[ $AV_STATUS2 -eq 0 ]] && grep -qE "PASS +acceptance\.landed +$AV_A" <<<"$AV_OUT2" && ! grep -q 'candidate-moved' <<<"$AV_OUT2"; then
    av_ok "--base sets aside what landed" "new since base: moved; on base: acceptance.landed"
else
    av_fail "--base sets aside what landed" "exit $AV_STATUS/$AV_STATUS2: $(av_lines "$AV_OUT") / $(av_lines "$AV_OUT2")"
fi

# ── OBL-005 (Q-001 (b)): only a re-verification of THIS candidate clears it ─
sleep 1   # a re-verification must be recorded after the resolution, to the second
av_verify not_established "src/helper.txt read at $AV_MOVED; not established"
av_commit "a verifier that did not re-establish"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/helper.txt'; then
    av_ok "not re-established does not clear" "still acceptance.candidate-moved (exit $AV_STATUS)"
else
    av_fail "not re-established does not clear" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
# A retained legacy verdict is history, not review of this candidate. It
# must not clear the move merely because its file and event were committed later.
sed 's|^evidence = .*|evidence = "legacy synthetic observation with no subject binding"|' \
    "$AV_TMP/verified.toml" > "$AV_TMP/legacy.toml"
AV_LEGACY=$(av_war verify "$AV_A" --response "$AV_TMP/legacy.toml" 2>&1)
AV_LEGACY_STATUS=$?
av_commit "retained legacy verdict, not candidate qualification"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_LEGACY_STATUS -ne 0 ]] && grep -q 'verify.subject-unbound' <<<"$AV_LEGACY" \
    && [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/helper.txt' \
    && [[ "$(av_sha)" == "$AV_BEFORE" ]]; then
    av_ok "unbound history cannot clear a move" "legacy retained; acceptance.candidate-moved; resolution untouched"
else
    av_fail "unbound history cannot clear a move" "ingest $AV_LEGACY_STATUS; acceptance $AV_STATUS: $(av_lines "$AV_OUT")"
fi

av_verify established "src/core.txt and src/helper.txt read at $AV_MOVED"
# The contract moves AFTER review, alongside the verdict commit. Own-record
# exclusions must not turn that unreviewed contract into an accepted candidate.
AV_INTENT="$PLANT_ROOT/docs/warrants/$AV_A/atoms/10-intent.md"
cp "$AV_INTENT" "$AV_TMP/reviewed-intent.md"
printf '\nA new required outcome was added after the review.\n' >> "$AV_INTENT"
av_commit "a changed contract alongside a verdict about the older contract"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/helper.txt'; then
    av_ok "a changed contract cannot borrow review" "the candidate contract differs from the bound review"
else
    av_fail "a changed contract cannot borrow review" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
cp "$AV_TMP/reviewed-intent.md" "$AV_INTENT"
av_commit "restore the exact reviewed contract"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -eq 0 ]] && grep -qE "PASS +acceptance\.unchanged +$AV_A: .*\(re-verified\)" <<<"$AV_OUT" && [[ "$(av_sha)" == "$AV_BEFORE" ]]; then
    av_ok "a re-verification clears it" "acceptance.unchanged (re-verified), resolution untouched"
else
    av_fail "a re-verification clears it" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
# Candidate facts come from Git, never from a dirty current checkout.
AV_REVIEWED=$(git -C "$PLANT_ROOT" rev-parse HEAD)
# Portable review retains the actual recorded evidence, not only its hash.
AV_OUT=$(av_war verify "$AV_A" --performer claude --bundle --json 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -eq 0 ]] && python3 - "$PLANT_ROOT" "$AV_A" "$AV_OUT" <<'PY_PACKET'
import hashlib, json, sys
from pathlib import Path
root, alias = Path(sys.argv[1]), sys.argv[2]
folder = root / "docs/warrants" / alias
index = json.loads(sys.argv[3])["result"]["packets"]
packets = [root / ref["path"] for ref in index]
# Older retained packets describe earlier inputs, never current bytes.
assert len(list((folder / "verifications").glob("bundle-*.json"))) >= len(packets)
assert packets
evidence = list((folder / "gate-runs").glob("*"))
inputs = [p for p in (root / "src").glob("*") if p.is_file()]
inputs.append(root / "support/api.txt")
assert len(evidence) >= 4 and len(inputs) >= 2
expected = [(p, "gate-evidence", "gate_evidence") for p in evidence]
for packet in packets:
    data = json.loads(packet.read_text())
    sources = {s["path"]: s for s in data["required_sources"]}
    assert data["request"]["reviewed_subject"]["gate_links"]["src/linked"] == "../support"
    for file in inputs:
        path = str(file.relative_to(root))
        assert data["request"]["reviewed_subject"]["gate_inputs"][path] == "sha256:" + hashlib.sha256(file.read_bytes()).hexdigest()
        assert path not in sources, "source inventory is not automatic reviewer context"
    for file, kind, binding in expected:
        path = str(file.relative_to(root))
        source = sources[path]
        assert source["kind"] == kind and source["present"]
        carried = source["text"].encode() if "text" in source else bytes(source["bytes"])
        assert carried == file.read_bytes(), path
        assert source["sha256"] == data["request"]["reviewed_subject"][binding][path]
PY_PACKET
then
    av_ok "portable packet retains gate evidence" "evidence bytes are carried; source inputs remain exactly bound"
else
    av_fail "portable packet retains gate evidence" "exit $AV_STATUS; required evidence bytes were not carried"
fi
# A bound review belongs to this Warrant and the verifier named by its record.
for AV_IDENTITY in actor_ref warrant_uuid; do
    python3 - "$PLANT_ROOT/docs/warrants/$AV_A/journal.jsonl" "$AV_IDENTITY" <<'PY_IDENTITY'
import json, sys
p, field = sys.argv[1:]
rows = [json.loads(line) for line in open(p) if line.strip()]
assert any(row.get("type") == "verification.recorded" for row in rows)
for row in rows:
    if row.get("type") == "verification.recorded":
        row[field] = "agent://different-verifier" if field == "actor_ref" else "00000000-0000-0000-0000-000000000000"
open(p, "w").write("".join(json.dumps(row) + "\n" for row in rows))
PY_IDENTITY
    av_commit "a reviewed event with wrong $AV_IDENTITY"
    AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
    if [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/helper.txt'; then
        av_ok "wrong $AV_IDENTITY cannot clear review" "the journal identity does not bind this verdict"
    else
        av_fail "wrong $AV_IDENTITY cannot clear review" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
    fi
    git -C "$PLANT_ROOT" reset -q --hard "$AV_REVIEWED"
done
# The verifier observed gate output as well as code and definitions.
python3 - "$PLANT_ROOT/docs/warrants/$AV_A/gate-runs" <<'PY_OUTPUT'
from pathlib import Path
import sys
files = list(Path(sys.argv[1]).glob("*.stdout.txt"))
assert len(files) == 1, files
with files[0].open("ab") as stream:
    stream.write(b"Changed evidence after independent review.\n")
PY_OUTPUT
av_commit "changed gate output after review"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/helper.txt'; then
    av_ok "changed gate output cannot borrow review" "the raw evidence differs from the reviewed snapshot"
else
    av_fail "changed gate output cannot borrow review" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
git -C "$PLANT_ROOT" reset -q --hard "$AV_REVIEWED"
printf '\nOnly the working tree has this unreviewed contract change.\n' >> "$AV_INTENT"
AV_OUT=$(av_war pins --candidate "$AV_REVIEWED" 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -eq 0 ]] && grep -qE "PASS +acceptance\.unchanged +$AV_A: .*\(re-verified\)" <<<"$AV_OUT"; then
    av_ok "candidate review ignores dirty checkout" "the exact Git candidate remains reviewed"
else
    av_fail "candidate review ignores dirty checkout" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
cp "$AV_TMP/reviewed-intent.md" "$AV_INTENT"

printf 'a second change\n' > "$PLANT_ROOT/src/second.txt"
av_commit "a second in-scope change"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
AV_LINE=$(grep 'acceptance.candidate-moved' <<<"$AV_OUT")
# A new selected input invalidates the whole earlier review snapshot. Its
# old observations remain history; comparison may return to the resolution,
# so the finding can also name the earlier helper change. The new move must
# still be refused and explicitly named.
if [[ $AV_STATUS -ne 0 ]] && grep -qF 'src/second.txt' <<<"$AV_LINE"; then
    av_ok "it clears for that candidate only" "a second commit moves it again: src/second.txt"
else
    av_fail "it clears for that candidate only" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi

# The checkout cannot shrink the candidate's declared gate-input scope.
AV_GATE="$PLANT_ROOT/docs/gates/plant.acceptance@1.0.0.yaml"
cp "$AV_GATE" "$AV_TMP/reviewed-gate.yaml"
sed -i 's|^inputs: .*|inputs: ["README.txt"]|' "$AV_GATE"
assert_present 'inputs: ["README.txt"]' "$AV_GATE"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/second.txt'; then
    av_ok "checkout cannot shrink candidate scope" "Git gate inputs still require src/second.txt"
else
    av_fail "checkout cannot shrink candidate scope" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
cp "$AV_TMP/reviewed-gate.yaml" "$AV_GATE"

# A pinned deliverable is Q-001 (a): a human re-accepts it; a verifier cannot.
printf 'rewritten\n' > "$PLANT_ROOT/src/core.txt"
av_commit "the pinned deliverable itself"
sleep 1
av_verify established "src/core.txt read after it was rewritten"
av_commit "re-verified after the pinned change"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && av_names 'acceptance.candidate-moved' "$AV_OUT" 'src/core.txt (D-001)' && grep -q 'Q-001 (a)' <<<"$AV_OUT"; then
    av_ok "a moved pin needs a human" "src/core.txt (D-001), Q-001 (a); re-verification did not clear it"
else
    av_fail "a moved pin needs a human" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
# ... and the human's re-acceptance of that pin — today a signed correction,
# the act the finding names — is what clears it. Signed by the plant key in a
# second throwaway agent that holds nothing else, exactly as the setup did.
if WAR="$REPO_ROOT/$WAR" D="$PLANT_ROOT" T="$AV_TMP" A="$AV_A" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$AV_TMP/correct.log" 2>&1 <<'CORRECT'
cd "$D"
eval "$(ssh-agent -s)" >/dev/null
trap 'ssh-agent -k >/dev/null 2>&1 || true' EXIT
ssh-add -q "$T/id_plant"
keys=$(ssh-add -l)
fp=$(ssh-keygen -lf "$T/id_plant.pub" | awk '{print $2}')
[[ $(grep -c . <<<"$keys") -eq 1 ]] && grep -qF -- "$fp" <<<"$keys" \
    || { echo "the throwaway agent holds more than the plant key: $keys"; exit 1; }
"$WAR" --root . sign "$A/D-001" --ssh-sign --as "Plant Signer" --kind behaviour-change </dev/null
CORRECT
then
    av_commit "the pinned deliverable re-accepted by correction"
    AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
    AV_CLEAR=$(grep -F "$AV_A" <<<"$AV_OUT" | grep -F 'acceptance.' || true)
    if [[ $AV_STATUS -eq 0 && "$AV_CLEAR" == *"acceptance.unchanged"* && "$AV_CLEAR" != *"acceptance.candidate-moved"* ]]; then
        av_ok "a signed correction clears the pin" "war sign $AV_A/D-001: acceptance.unchanged at the re-accepted candidate"
    else
        av_fail "a signed correction clears the pin" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
    fi
else
    av_fail "a signed correction clears the pin" "the correction was not recorded: $(tail -3 "$AV_TMP/correct.log" | tr '\n' '|')"
fi
git -C "$PLANT_ROOT" reset -q --hard "$AV_MOVED"
git -C "$PLANT_ROOT" clean -fdq

# ── OBL-003: history that cannot answer is UNKNOWN, never unchanged ─────────
# The readable locator above answered; the same resolution with its locator
# taken away, or pointed at a commit this repository does not hold, cannot.
python3 - "$AV_RES" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p).read()
open(p, "w").write(re.sub(r'\n\[locator\]\n(?:(?!\[).*\n?)*', '\n', s))
PY
assert_gone '[locator]' "$AV_RES"
av_commit "candidate with no recorded locator"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && grep -qE "UNKNOWN +acceptance\.unknown +$AV_A: UNKNOWN \(no locator" <<<"$AV_OUT" \
    && ! grep -qE 'acceptance\.(unchanged|candidate-moved)' <<<"$AV_OUT" && ! grep -qE '^ERROR' <<<"$AV_OUT"; then
    av_ok "no locator is UNKNOWN" "acceptance.unknown (no locator), no error, not unchanged"
else
    av_fail "no locator is UNKNOWN" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
git -C "$PLANT_ROOT" reset -q --hard "$AV_MOVED"
sed -i 's/^commit_sha = ".*"/commit_sha = "0123456789abcdef0123456789abcdef01234567"/' "$AV_RES"
assert_present '0123456789abcdef0123456789abcdef01234567' "$AV_RES"
av_commit "candidate with an unreadable recorded locator"
AV_OUT=$(av_war pins --candidate HEAD 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && grep -qE "UNKNOWN +acceptance\.unknown +$AV_A: UNKNOWN \(commit 0123456789ab not readable" <<<"$AV_OUT" \
    && ! grep -qE 'acceptance\.(unchanged|candidate-moved)' <<<"$AV_OUT" && ! grep -qE '^ERROR' <<<"$AV_OUT"; then
    av_ok "an unreadable locator is UNKNOWN" "acceptance.unknown (commit not readable), no error"
else
    av_fail "an unreadable locator is UNKNOWN" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi
git -C "$PLANT_ROOT" reset -q --hard "$AV_MOVED"
AV_OUT=$(av_war pins --candidate no-such-revision 2>&1); AV_STATUS=$?
if [[ $AV_STATUS -ne 0 ]] && grep -qE 'UNKNOWN +acceptance\.unknown +candidate no-such-revision' <<<"$AV_OUT" && ! grep -q 'acceptance.unchanged' <<<"$AV_OUT"; then
    av_ok "an unreadable candidate is UNKNOWN" "acceptance.unknown, nothing reported unchanged"
else
    av_fail "an unreadable candidate is UNKNOWN" "exit $AV_STATUS: $(av_lines "$AV_OUT")"
fi

command rm -rf "$AV_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT