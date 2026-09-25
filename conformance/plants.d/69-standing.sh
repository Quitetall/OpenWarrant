# shellcheck shell=bash
# OW-WAR-0142 — a standing authorization (OW-ADR-0029): one signature
# pre-authorizes a class of routine work.
#
# Three scratch programs, a key generated here and a throwaway ssh-agent
# asserted to hold only it (as 49 and 96-batch). Nothing here is the owner's
# key, and nothing depends on this repository's own Warrants.
#
#   SA  OBL-003  a class that names or matches an authority path is refused
#   SB  OBL-002  inside the class: authorized from the signature; outside:
#                refused by term, nothing written
#       OBL-004  a class widens only by a new human signature
#       OBL-006  routine work never takes a file from work in flight
#   SC  OBL-005  a resolution is never automatic
#       OBL-007  the dialog count, A against B
#       OBL-006  a resolved owner's path is taken, and the pin is historical
#
# Every claim is paired with a refusal, and every refusal checks that
# nothing was written.

echo "== standing authorization (OW-WAR-0142) =="
ST_TMP=$(mktemp -d)
ssh-keygen -q -t ed25519 -N "" -C plant -f "$ST_TMP/id_plant"
ST_PUB=$(cut -d' ' -f1,2 "$ST_TMP/id_plant.pub")
ST_OLD_SOCK=${SSH_AUTH_SOCK:-}
eval "$(ssh-agent -s > "$ST_TMP/agent.env"; cat "$ST_TMP/agent.env")" >/dev/null
ssh-add -q "$ST_TMP/id_plant" 2>/dev/null
# The agent must hold this key and nothing else.
ST_KEYS=$(ssh-add -l 2>/dev/null)
ST_FP=$(ssh-keygen -lf "$ST_TMP/id_plant.pub" | awk '{print $2}')
if [[ $(grep -c . <<<"$ST_KEYS") -ne 1 ]] || ! grep -qF -- "$ST_FP" <<<"$ST_KEYS"; then
    printf 'PLANT SETUP FAILED: the throwaway agent holds more than the plant key:\n%s\n' "$ST_KEYS" >&2
    ssh-agent -k >/dev/null 2>&1
    exit 9
fi

# Every `ssh-keygen -Y sign` a `war` makes is counted here (OBL-007); every
# other call passes through untouched.
mkdir -p "$ST_TMP/bin"
ST_REAL_KEYGEN=$(command -v ssh-keygen)
cat > "$ST_TMP/bin/ssh-keygen" <<SHIM
#!/usr/bin/env bash
if [[ " \$* " == *" -Y sign "* ]]; then echo sign >> "$ST_TMP/signs.log"; fi
exec "$ST_REAL_KEYGEN" "\$@"
SHIM
chmod +x "$ST_TMP/bin/ssh-keygen"
# `war evidence record` runs the program's gate, whose argv is `war check`.
ln -sf "$(realpath "$WAR")" "$ST_TMP/bin/war"
touch "$ST_TMP/signs.log"
st_signs() { grep -c . "$ST_TMP/signs.log"; }

st_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
st_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
st_errors() { grep -E '^(ERROR|WARN|UNKNOWN)' <<<"$1" | head -3 | tr '\n' '|'; }
st_tree() { (cd "$1" && find . -type f -print0 | sort -z | xargs -0 sha256sum) | sha256sum | cut -d' ' -f1; }
st_commit() { git -C "$1" add -A >/dev/null 2>&1; git -C "$1" -c user.email=plant@invalid -c user.name=plant commit -qm "$2" >/dev/null 2>&1; }

# st_corpus <NS> → root. The plant signer is the only human who may sign;
# `plant-agent` is an agent holding the authorizer role (refused by kind),
# `claude` is the performer as a human authorizer (refused as SelfAct), and
# `plant-policy` is §27.3's policy service. The gate declares its inputs, so
# a drafted response is not a change to what it read.
st_corpus() {
    local root
    root=$(scratch_corpus "$1")
    [[ -d "${root:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
    printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$ST_PUB" > "$root/docs/authority/allowed_signers"
    cat > "$root/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/69-standing.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while these plants run."
ssh_principal = "plant"

[[assignment]]
actor = "plant-agent"
actor_kind = "agent"
roles = ["authorizer", "performer"]
assigned_by = "conformance/plants.d/69-standing.sh"
effective_time = "2026-01-01T00:00:00Z"

[[assignment]]
actor = "claude"
actor_kind = "human"
roles = ["authorizer"]
assigned_by = "conformance/plants.d/69-standing.sh"
effective_time = "2026-01-01T00:00:00Z"

[[assignment]]
actor = "plant-policy"
actor_kind = "policyservice"
roles = ["resolver"]
assigned_by = "conformance/plants.d/69-standing.sh"
effective_time = "2026-01-01T00:00:00Z"
ROLES
    sed -i 's|^argv: \["war", "check", "--generated"\]$|argv: ["war", "check", "--generated"]\ninputs: ["src/**", "lib/**"]|' \
        "$root/docs/gates/software.repo.war-check@1.0.0.yaml"
    assert_present 'inputs: ["src/**", "lib/**"]' "$root/docs/gates/software.repo.war-check@1.0.0.yaml"
    sed -i 's|^allow_automated_resolution = false$|allow_automated_resolution = true|' "$root/openwarrant.toml"
    assert_present 'allow_automated_resolution = true' "$root/openwarrant.toml"
    mkdir -p "$root/src" "$root/lib"
    printf 'fn shared() {}\n' > "$root/src/shared.rs"
    "$WAR" --root "$root" compile >/dev/null 2>&1
    st_commit "$root" "register, a gate that declares its inputs, src/"
    printf '%s' "$root"
}

# st_class <file> <id> <rev> <paths, as TOML array items> [<expires_at>] [<max_warrants>] [<extra line>]
st_class() {
    local expires=${5:-$(date -u -d '+30 days' +%Y-%m-%dT%H:%M:%SZ)}
    cat > "$1" <<CLS
schema = "oh.war/standing-authorization/v1"
id = "$2"
revision = $3
meaning = "Routine edits the plant signs once."
paths = [$4]
profile = "delivery"
assurance = "basic"
gates = ["gate://ops.conformance.plants@1.1.0", "gate://software.repo.war-check@1.0.0"]
expires_at = "$expires"
max_warrants = ${6:-10}
${7:-}

[budget]
budget_tokens = 24000
wall_time_seconds = 1800
max_stages = 4
max_deliverables = 4
CLS
}

# st_warrant <root> <class ref or ""> <title> <path>... → alias. A routine
# Warrant: filled atoms, one agent stage with a budget, one obligation citing
# war-check, the paths declared (and created), the class in the manifest.
st_warrant() {
    local root=$1 ref=$2 title=$3 alias dir uuid i
    shift 3
    [[ -d "${root:-}/.git" ]] || { printf 'PLANT SETUP FAILED: st_warrant outside a scratch corpus\n' >&2; exit 9; }
    alias=$("$WAR" --root "$root" new "$title" 2>/dev/null | sed -n 's|^created docs/warrants/\([A-Z]*-WAR-[0-9]*\) .*|\1|p')
    [[ -n "$alias" ]] || { printf 'PLANT SETUP FAILED: war new %s\n' "$title" >&2; exit 9; }
    dir="$root/docs/warrants/$alias"
    uuid=$(sed -n 's/^uuid = "\(.*\)"/\1/p' "$dir/manifest.toml")
    st_atom() { # file role order body
        printf -- '---\nschema: oh.war/atom/v1\nwarrant_uuid: %s\nrole: %s\njurisdiction: authored\norder: %s\nclassification: internal\n---\n\n%s\n' "$uuid" "$2" "$3" "$4" > "$dir/atoms/$1"
    }
    st_atom 10-intent.md intent 10 "# Intent

## Problem

$title.

## Desired Outcome

The declared files carry the change.

## Out of scope

Everything else."
    st_atom 20-basis.md basis 20 "# Basis

## Governing sources

- The program's SAS.

## Unknowns

- None."
    st_atom 40-work-order.md work_order 40 "# Work Order

## Deliverables

1. The declared files.

## Frozen Surfaces

Everything not declared.

## Premade Instructions

- Never edit a record to make a checker green.

## Autonomy and Escalation

Tier T2.

## Rollback

Revert the commit."
    cat > "$dir/atoms/45-milestones.yaml" <<'MS'
schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "The change lands"
    stage_refs: ["STAGE-001"]
    obligation_refs: ["OBL-001"]

stages:
  - id: "STAGE-001"
    title: "Make the change"
    executor_kind: "agent"
    executor_ref: "agent://plant"
    responsibility_tier: "T2"
    budget_tokens: 12000
MS
    st_atom 60-assurance.md assurance 60 "# Assurance

## Acceptance Obligations

### OBL-001 — the declared files carry the change
- **scope:** the declared files only.
- **gate:** \`gate://software.repo.war-check@1.0.0\`
- **evidence:** the files, and \`war check\` clean.

## Gate Adequacy

Required at \`basic\`.

**Adversarial question:** could this look done and not be? Only if the
files were empty; the verifier reads them.

- **outcome:** no_counterexample"
    printf 'schema = "oh.war/rationale/v1"\n' > "$dir/rationale.toml"
    {
        printf 'schema = "oh.war/deliverables/v1"\n'
        i=0
        for p in "$@"; do
            i=$((i + 1))
            mkdir -p "$root/$(dirname "$p")"
            [[ -f "$root/$p" ]] || printf '// %s\n' "$p" > "$root/$p"
            printf '\n[[deliverable]]\nid = "D-%03d"\ntitle = "%s"\nkind = "file"\ntarget_ref = "%s"\nrequired = true\ncontent_addressed = true\nprovenance_required = true\nobligation_refs = ["OBL-001"]\n' "$i" "$p" "$p"
            printf '\n[deliverable.provenance]\nproducer = "claude"\nproducing_attempt = "plant/attempt-1"\ncontract_digest = "unrecorded"\ntool_or_runtime_identity = "plant"\ncreation_method = "authored"\ncontent_digest = "sha256:%064d"\nmedia_type = "text/x-rust"\nclassification = "internal"\nretention = "repository-lifetime"\nsource_holder = "git"\n' 0
        done
    } > "$dir/deliverables.toml"
    [[ -n "$ref" ]] && printf '\n[standing]\nref = "%s"\n' "$ref" >> "$dir/manifest.toml"
    "$WAR" --root "$root" pins --refresh --alias "$alias" >/dev/null 2>&1
    "$WAR" --root "$root" compile >/dev/null 2>&1
    printf '%s' "$alias"
}

# st_resolvable <root> <alias>...: an independent verification of OBL-001,
# committed, then the cited gate run over the committed tree and committed.
st_resolvable() {
    local root=$1 a
    shift
    for a in "$@"; do
        "$WAR" --root "$root" pins --refresh --alias "$a" >/dev/null 2>&1
        cat > "$ST_TMP/verification-$a.toml" <<VERIFY
schema = "oh.war/verification-response/v1"
warrant = "$a"

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
        "$WAR" --root "$root" verify "$a" --response "$ST_TMP/verification-$a.toml" >/dev/null 2>&1
    done
    "$WAR" --root "$root" compile >/dev/null 2>&1
    st_commit "$root" "verified"
    # One gate run per Warrant, each over projections brought up to date:
    # a run records evidence the corpus status projects, and the next run's
    # `war check --generated` would otherwise see the last one's drift.
    for a in "$@"; do
        PATH="$ST_TMP/bin:$PATH" "$WAR" --root "$root" evidence record "$a" >/dev/null 2>&1
        "$WAR" --root "$root" compile >/dev/null 2>&1
    done
    st_commit "$root" "evidence"
}

st_sign() { # root target [flags] — one act, ssh-signed as the plant signer
    local root=$1 target=$2
    shift 2
    PATH="$ST_TMP/bin:$PATH" "$WAR" --root "$root" sign "$target" --ssh-sign --as "Plant Signer" "$@" </dev/null 2>&1
}
st_sign_as() { # root target actor — one act, ssh-signed as someone else
    PATH="$ST_TMP/bin:$PATH" "$WAR" --root "$1" sign "$2" --ssh-sign --as "$3" </dev/null 2>&1
}

# ── SA · OBL-003: a class that names or matches an authority path ──────────
PLANT_ROOT=$(st_corpus SA)
# Sourced outside plant.sh, or with a setup that failed inside the command
# substitution, PLANT_ROOT is empty and every `--root` below would name the
# real repository. Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus for SA\n' >&2; exit 9; }
SA=$PLANT_ROOT
SA_STANDING="$SA/docs/authority/standing"
for bad in "docs/authority/roles.toml" "docs/authority/allowed_signers" "openwarrant.toml" \
    "crates/openwarrant-cli/src/sign.rs" "**" "docs/**"; do
    st_class "$ST_TMP/bad.toml" "bad" 1 "\"$bad\""
    SA_BEFORE=$(st_tree "$SA/docs")
    SA_OUT=$("$WAR" --root "$SA" standing propose "$ST_TMP/bad.toml" 2>&1); SA_STATUS=$?
    if [[ $SA_STATUS -ne 0 ]] && grep -E '^ERROR +standing\.never-coverable' <<<"$SA_OUT" | grep -qF -- "\`$bad\`" \
        && [[ "$(st_tree "$SA/docs")" == "$SA_BEFORE" ]]; then
        st_ok "propose refuses $bad" "standing.never-coverable, nothing written"
    else
        st_fail "propose refuses $bad" "exit $SA_STATUS: $(st_errors "$SA_OUT")"
    fi
    # The same class placed by hand: the signature is refused too, by name,
    # and the dry run writes nothing.
    mkdir -p "$SA_STANDING"
    cp "$ST_TMP/bad.toml" "$SA_STANDING/bad@1.toml"
    SA_BEFORE=$(st_tree "$SA/docs")
    SA_OUT=$("$WAR" --root "$SA" sign standing:bad@1 --as "Plant Signer" --dry-run 2>&1); SA_STATUS=$?
    if [[ $SA_STATUS -ne 0 ]] && grep -q 'standing.never-coverable' <<<"$SA_OUT" && grep -qF -- "\`$bad\`" <<<"$SA_OUT" \
        && ! grep -q 'would-record' <<<"$SA_OUT" && [[ "$(st_tree "$SA/docs")" == "$SA_BEFORE" ]]; then
        st_ok "sign --dry-run refuses $bad" "standing.never-coverable, nothing written"
    else
        st_fail "sign --dry-run refuses $bad" "exit $SA_STATUS: $(st_errors "$SA_OUT")"
    fi
    command rm -f "$SA_STANDING/bad@1.toml"
done
# The positive control: the refusal is not a refusal of everything.
st_class "$ST_TMP/tui.toml" "tui" 1 '"crates/openwarrant-cli/src/tui/**"'
SA_OUT=$("$WAR" --root "$SA" standing propose "$ST_TMP/tui.toml" 2>&1); SA_STATUS=$?
SA_DRY=$("$WAR" --root "$SA" sign standing:tui@1 --as "Plant Signer" --dry-run 2>&1); SA_DRY_STATUS=$?
if [[ $SA_STATUS -eq 0 ]] && grep -q 'standing.proposed' <<<"$SA_OUT" && [[ -f "$SA_STANDING/tui@1.toml" ]] \
    && [[ $SA_DRY_STATUS -eq 0 ]] && grep -q 'standing.would-record' <<<"$SA_DRY"; then
    st_ok "a tui/** class is accepted" "standing.proposed, sign --dry-run would-record"
else
    st_fail "a tui/** class is accepted" "exit $SA_STATUS/$SA_DRY_STATUS: $(st_errors "$SA_OUT$SA_DRY")"
fi
# The never-coverable set is a constant in standing.rs, read from nowhere.
st_extends() { # file → 0 when the file could read something that extends the set
    grep -nE 'std::fs|include_str!|include_bytes!|env::var|read_to_string|read_dir|NEVER_COVERABLE[^;]*(push|extend|insert)' "$1" \
        | grep -vE '^[0-9]+: *//' >/dev/null
}
SA_SRC="$REPO_ROOT/crates/openwarrant-core/src/standing.rs"
# Captured first: under pipefail a negated pipeline whose `grep -q` stops
# early would read as "not found".
SA_ELSEWHERE=$(grep -rn 'NEVER_COVERABLE' "$REPO_ROOT/crates" --include='*.rs' | grep -v 'crates/openwarrant-core/src/standing.rs')
cp "$SA_SRC" "$ST_TMP/standing-planted.rs"
printf '\nfn planted() -> String { std::fs::read_to_string("never.toml").unwrap() }\n' >> "$ST_TMP/standing-planted.rs"
if grep -q '^pub const NEVER_COVERABLE: &\[&str\] = &\[' "$SA_SRC" && ! st_extends "$SA_SRC" \
    && st_extends "$ST_TMP/standing-planted.rs" \
    && ! grep -qE 'push|extend|insert|= *&' <<<"$SA_ELSEWHERE"; then
    st_ok "NEVER_COVERABLE is a constant" "no file read in standing.rs; a planted read is caught"
else
    st_fail "NEVER_COVERABLE is a constant" "the grep did not hold, or missed the planted read"
fi
corpus_gone "$SA"
unset PLANT_ROOT

# ── SB · OBL-002, OBL-004, OBL-006 ───────────────────────────────────────────
PLANT_ROOT=$(st_corpus SB)
# Sourced outside plant.sh, or with a setup that failed inside the command
# substitution, PLANT_ROOT is empty and every `--root` below would name the
# real repository. Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus for SB\n' >&2; exit 9; }
SB=$PLANT_ROOT
sb() { "$WAR" --root "$SB" "$@"; }
st_class "$ST_TMP/routine1.toml" "routine" 1 '"src/**"'
sb standing propose "$ST_TMP/routine1.toml" >/dev/null 2>&1
SB_OUT=$(st_sign "$SB" standing:routine@1); SB_STATUS=$?
if [[ $SB_STATUS -eq 0 ]] && grep -q 'standing.accepted' <<<"$SB_OUT" \
    && [[ -f "$SB/docs/authority/responses/STANDING-routine@1.response.toml.sig" ]]; then
    st_ok "the owner signs the class" "standing.accepted, signed and attested"
else
    st_fail "the owner signs the class" "exit $SB_STATUS: $(st_errors "$SB_OUT")"
fi

# OBL-002, the positive: inside, authorized from the owner's signature.
SB_W1=$(st_warrant "$SB" "standing://routine@1" "A routine change" src/a.rs)
SB_W1_DIR="$SB/docs/warrants/$SB_W1"
SB_DIGEST=$(sb authorize "$SB_W1" 2>/dev/null | sed -n 's/^contract_digest = "\(.*\)"/\1/p')
SB_OUT=$(sb standing apply "$SB_W1" 2>&1); SB_STATUS=$?
SB_AUTH="$SB_W1_DIR/authorization.toml"
SB_CHECK=$(sb check 2>&1)
if [[ $SB_STATUS -eq 0 ]] && grep -q 'standing.recorded' <<<"$SB_OUT" \
    && grep -qF "contract_digest = \"$SB_DIGEST\"" "$SB_AUTH" \
    && grep -qF 'authorizer = "Plant Signer"' "$SB_AUTH" \
    && grep -qF 'policy_basis = "standing://routine@1"' "$SB_AUTH" \
    && grep -qF 'target_ref = "src/a.rs"' "$SB_AUTH" \
    && grep -qE "^PASS +standing\.inside-class .*$SB_W1" <<<"$SB_CHECK" \
    && grep -qE "^PASS +authority\.signed +$SB_W1" <<<"$SB_CHECK" \
    && ! grep -q '^ERROR' <<<"$SB_CHECK"; then
    st_ok "inside the class: authorized" "its digest, the signer, standing://routine@1, its set; check passes"
else
    st_fail "inside the class: authorized" "exit $SB_STATUS: $(st_errors "$SB_OUT$(grep '^ERROR' <<<"$SB_CHECK")")"
fi
# ...and it can start at once: a dispatch needs no further signature.
SB_OUT=$(sb dispatch "$SB_W1" STAGE-001 2>&1)
if grep -q 'dispatch.compiled' <<<"$SB_OUT"; then
    st_ok "a covered Warrant starts at once" "dispatch.compiled, no signature asked"
else
    st_fail "a covered Warrant starts at once" "$(st_errors "$SB_OUT")"
fi
st_commit "$SB" "W1 covered"

# OBL-002, the refusals: each planted Warrant refused by its own term, its
# directory byte-identical before and after.
st_refused() { # name alias rule
    local dir="$SB/docs/warrants/$2" before out status
    before=$(st_tree "$dir")
    out=$(sb standing apply "$2" 2>&1); status=$?
    if [[ $status -ne 0 ]] && grep -qE "^ERROR +$3 " <<<"$out" && [[ ! -f "$dir/authorization.toml" ]] \
        && [[ "$(st_tree "$dir")" == "$before" ]]; then
        st_ok "$1" "$3, nothing written"
    else
        st_fail "$1" "exit $status, wanted $3: $(st_errors "$out")"
    fi
}
SB_X=$(st_warrant "$SB" "standing://routine@1" "Outside the globs" src/b.rs lib/x.rs)
st_refused "a path outside the class" "$SB_X" 'standing\.outside-class'
SB_X=$(st_warrant "$SB" "standing://routine@1" "Controlled" src/c.rs)
sed -i 's/^assurance_level = "basic"$/assurance_level = "controlled"/' "$SB/docs/warrants/$SB_X/manifest.toml"
assert_present 'assurance_level = "controlled"' "$SB/docs/warrants/$SB_X/manifest.toml"
st_refused "assurance controlled" "$SB_X" 'standing\.assurance'
# st_adr <alias> <n>: an ADR atom composed into the Warrant.
st_adr() {
    local dir="$SB/docs/warrants/$1" uuid
    uuid=$(sed -n 's/^uuid = "\(.*\)"/\1/p' "$dir/manifest.toml")
    mkdir -p "$SB/docs/adr/atoms"
    cat > "$SB/docs/adr/atoms/SB-ADR-000$2-plant.md" <<ADR
---
schema: oh.war/atom/v1
adr_uuid: 01a0d289-1e89-7990-8b2c-5a43577b5c0$2
local_alias: SB-ADR-000$2
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://$uuid"
---

# ADR SB-000$2: a decision a class cannot carry

## Status

Proposed.

## Context

The plant.

## Decision

None.

## Consequences

None.
ADR
    printf '\n[[atoms]]\nordinal = 30\nrole = "adr"\npath = "../../adr/atoms/SB-ADR-000%s-plant.md"\nrequired = true\n' "$2" >> "$dir/manifest.toml"
    assert_present "SB-ADR-000$2-plant.md" "$dir/manifest.toml"
}
SB_X=$(st_warrant "$SB" "standing://routine@1" "A decision" src/d.rs)
st_adr "$SB_X" 1
sed -i 's/^profile = "delivery"$/profile = "decision"/' "$SB/docs/warrants/$SB_X/manifest.toml"
assert_present 'profile = "decision"' "$SB/docs/warrants/$SB_X/manifest.toml"
st_refused "profile decision" "$SB_X" 'standing\.profile'
SB_X=$(st_warrant "$SB" "standing://routine@1" "Carries an ADR" src/e.rs)
st_adr "$SB_X" 2
st_refused "an adr atom" "$SB_X" 'standing\.adr'
SB_X=$(st_warrant "$SB" "standing://routine@1" "Accepts a risk" src/f.rs)
cat >> "$SB/docs/warrants/$SB_X/rationale.toml" <<'RISK'

[[assumption]]
id = "A-001"
statement = "The plant's risk."
epistemic_status = "accepted_residual_risk"
consequence_if_false = "The plant is wrong."
judgment_ref = ""
RISK
st_refused "an accepted residual risk" "$SB_X" 'standing\.residual-risk'
SB_X=$(st_warrant "$SB" "standing://routine@1" "Another gate" src/g.rs)
sed -i 's|^- \*\*gate:\*\* `gate://software.repo.war-check@1.0.0`$|- **gate:** `gate://software.repo.war-check@1.0.0`, `gate://plant.other@1.0.0`|' "$SB/docs/warrants/$SB_X/atoms/60-assurance.md"
assert_present 'gate://plant.other@1.0.0' "$SB/docs/warrants/$SB_X/atoms/60-assurance.md"
st_refused "a gate the class does not name" "$SB_X" 'standing\.gate'
SB_X=$(st_warrant "$SB" "standing://routine@1" "Over budget" src/h.rs)
sed -i 's/^    budget_tokens: 12000$/    budget_tokens: 24001/' "$SB/docs/warrants/$SB_X/atoms/45-milestones.yaml"
assert_present 'budget_tokens: 24001' "$SB/docs/warrants/$SB_X/atoms/45-milestones.yaml"
st_refused "a stage over budget_tokens" "$SB_X" 'standing\.budget'
SB_X=$(st_warrant "$SB" "standing://routine@1" "Over wall time" src/i.rs)
sed -i 's/^    budget_tokens: 12000$/    budget_tokens: 12000\n    wall_time_seconds: 1801/' "$SB/docs/warrants/$SB_X/atoms/45-milestones.yaml"
assert_present 'wall_time_seconds: 1801' "$SB/docs/warrants/$SB_X/atoms/45-milestones.yaml"
st_refused "a stage over wall_time_seconds" "$SB_X" 'standing\.budget'
# Past expires_at: a class signed to expire seconds from now.
st_class "$ST_TMP/short.toml" "short" 1 '"src/**"' "$(date -u -d '+6 seconds' +%Y-%m-%dT%H:%M:%SZ)"
sb standing propose "$ST_TMP/short.toml" >/dev/null 2>&1
st_sign "$SB" standing:short@1 >/dev/null
SB_X=$(st_warrant "$SB" "standing://short@1" "After expiry" src/j.rs)
SB_OUT=$(sb standing apply "$SB_X" --dry-run 2>&1)
grep -q 'standing.would-record' <<<"$SB_OUT" && SB_SHORT_OK=1 || SB_SHORT_OK=0
sleep 8
if [[ $SB_SHORT_OK -eq 1 ]]; then
    st_refused "an application after expires_at" "$SB_X" 'standing\.expired'
else
    st_fail "an application after expires_at" "the class never covered anything before it expired"
fi
# Past max_warrants: a class that covers one.
st_class "$ST_TMP/one.toml" "one" 1 '"src/**"' "" 1
sb standing propose "$ST_TMP/one.toml" >/dev/null 2>&1
st_sign "$SB" standing:one@1 >/dev/null
SB_X=$(st_warrant "$SB" "standing://one@1" "The first of one" src/k.rs)
SB_OUT=$(sb standing apply "$SB_X" 2>&1)
SB_X=$(st_warrant "$SB" "standing://one@1" "The second of one" src/l.rs)
if grep -q 'standing.recorded' <<<"$SB_OUT"; then
    st_refused "an application past max_warrants" "$SB_X" 'standing\.exhausted'
else
    st_fail "an application past max_warrants" "the first was not covered: $(st_errors "$SB_OUT")"
fi

# OBL-002: a covered Warrant then edited outside the class is not PASS.
cp "$SB_W1_DIR/deliverables.toml" "$ST_TMP/w1-deliverables.toml"
printf '\n[[deliverable]]\nid = "D-009"\ntitle = "outside"\nkind = "file"\ntarget_ref = "lib/outside.rs"\nrequired = false\ncontent_addressed = false\nprovenance_required = false\nobligation_refs = ["OBL-001"]\n' >> "$SB_W1_DIR/deliverables.toml"
SB_CHECK=$(sb check 2>&1); SB_STATUS=$?
if [[ $SB_STATUS -ne 0 ]] && grep -E '^ERROR +standing\.outside-class' <<<"$SB_CHECK" | grep -q "$SB_W1" \
    && ! grep -qE "^PASS +standing\.inside-class .*$SB_W1" <<<"$SB_CHECK"; then
    st_ok "edited outside the class: ERROR" "standing.outside-class, not PASS"
else
    st_fail "edited outside the class: ERROR" "exit $SB_STATUS: $(grep -E 'standing' <<<"$SB_CHECK" | head -2 | tr '\n' '|')"
fi
cp "$ST_TMP/w1-deliverables.toml" "$SB_W1_DIR/deliverables.toml"

# OBL-004: one glob edited in the signed class file — unsigned, and nothing
# is covered under it.
SB_CLASS="$SB/docs/authority/standing/routine@1.toml"
cp "$SB_CLASS" "$ST_TMP/routine1.signed"
sed -i 's|^paths = \["src/\*\*"\]$|paths = ["src/**", "lib/**"]|' "$SB_CLASS"
assert_present 'paths = ["src/**", "lib/**"]' "$SB_CLASS"
SB_Y=$(st_warrant "$SB" "standing://routine@1" "Under an edited class" src/m.rs)
SB_CHECK=$(sb check 2>&1); SB_CHECK_STATUS=$?
SB_Y_BEFORE=$(st_tree "$SB/docs/warrants/$SB_Y")
SB_OUT=$(sb standing apply "$SB_Y" 2>&1); SB_STATUS=$?
if [[ $SB_CHECK_STATUS -ne 0 ]] && grep -qE '^ERROR +standing\.unsigned' <<<"$SB_CHECK" \
    && grep -qE "^ERROR +authority\.unsigned +$SB_W1" <<<"$SB_CHECK" \
    && [[ $SB_STATUS -ne 0 ]] && grep -qE '^ERROR +standing\.unsigned' <<<"$SB_OUT" \
    && [[ "$(st_tree "$SB/docs/warrants/$SB_Y")" == "$SB_Y_BEFORE" ]]; then
    st_ok "an edited glob unsigns the class" "standing.unsigned in check and at apply; $SB_W1 no longer signed"
else
    st_fail "an edited glob unsigns the class" "check $SB_CHECK_STATUS, apply $SB_STATUS: $(st_errors "$SB_OUT$(grep -E '^ERROR' <<<"$SB_CHECK")")"
fi
cp "$ST_TMP/routine1.signed" "$SB_CLASS"
# A class nobody signed covers nothing.
st_class "$ST_TMP/unsigned.toml" "unsigned" 1 '"src/**"'
sb standing propose "$ST_TMP/unsigned.toml" >/dev/null 2>&1
SB_Z=$(st_warrant "$SB" "standing://unsigned@1" "Under an unsigned class" src/n.rs)
st_refused "a class with no signature" "$SB_Z" 'standing\.unsigned'
# The signature is a human's: an agent is refused by kind, the performer as
# SelfAct, and nothing is written either way.
SB_RESP=$(ls "$SB/docs/authority/responses" | sort | sha256sum)
SB_OUT=$(st_sign_as "$SB" standing:unsigned@1 plant-agent); SB_STATUS=$?
SB_OUT2=$(st_sign_as "$SB" standing:unsigned@1 claude); SB_STATUS2=$?
if [[ $SB_STATUS -ne 0 ]] && grep -qE '^ERROR +standing\.not-permitted' <<<"$SB_OUT" && grep -q 'AgentProhibited' <<<"$SB_OUT" \
    && [[ $SB_STATUS2 -ne 0 ]] && grep -qE '^ERROR +standing\.not-permitted' <<<"$SB_OUT2" && grep -q 'SelfAct' <<<"$SB_OUT2" \
    && [[ "$(ls "$SB/docs/authority/responses" | sort | sha256sum)" == "$SB_RESP" ]]; then
    st_ok "an agent or the performer signs" "refused by kind (AgentProhibited) and as SelfAct; nothing written"
else
    st_fail "an agent or the performer signs" "exit $SB_STATUS/$SB_STATUS2: $(st_errors "$SB_OUT$SB_OUT2")"
fi
# The control for the refusal above: the plant signer's dry run of the same
# act would record.
SB_OUT=$(sb sign standing:unsigned@1 --as "Plant Signer" --dry-run 2>&1)
if grep -q 'standing.would-record' <<<"$SB_OUT"; then
    st_ok "the human's dry run would record" "standing.would-record"
else
    st_fail "the human's dry run would record" "$(st_errors "$SB_OUT")"
fi
# No MCP tool accepts, revokes or ingests a class.
SB_MCP=$(sb mcp --describe 2>&1)
SB_TOOLS=$(sed -n '1,/never registered/p' <<<"$SB_MCP")
SB_REFUSED=$(sed -n '/never registered/,$p' <<<"$SB_MCP")
if grep -qE '^  war_standing_apply ' <<<"$SB_MCP" && grep -qE '^  war_standing_show ' <<<"$SB_MCP" \
    && grep -qx '  war_standing_propose_ingest' <<<"$SB_REFUSED" \
    && grep -qx '  war_standing_accept' <<<"$SB_REFUSED" \
    && grep -qx '  war_standing_revoke' <<<"$SB_REFUSED" \
    && ! grep -qE '^  war_standing_(accept|revoke|propose|ingest)' <<<"$SB_TOOLS"; then
    st_ok "no MCP tool signs a class" "apply and show listed; accept, revoke, propose --ingest refused"
else
    st_fail "no MCP tool signs a class" "$(grep standing <<<"$SB_MCP" | tr '\n' '|')"
fi
# A new revision signed by the human covers Warrants applied after it; the
# Warrant covered under revision 1 keeps revision 1 (§31).
SB_LIB=$(st_warrant "$SB" "standing://routine@1" "Under lib" lib/y.rs)
SB_OUT_R1=$(sb standing apply "$SB_LIB" --dry-run 2>&1); SB_STATUS_R1=$?
st_class "$ST_TMP/routine2.toml" "routine" 2 '"src/**", "lib/**"'
sb standing propose "$ST_TMP/routine2.toml" >/dev/null 2>&1
st_sign "$SB" standing:routine@2 >/dev/null
sed -i 's|^ref = "standing://routine@1"$|ref = "standing://routine@2"|' "$SB/docs/warrants/$SB_LIB/manifest.toml"
SB_OUT=$(sb standing apply "$SB_LIB" 2>&1); SB_STATUS=$?
if [[ $SB_STATUS_R1 -ne 0 ]] && grep -q 'standing.outside-class' <<<"$SB_OUT_R1" \
    && [[ $SB_STATUS -eq 0 ]] && grep -qF 'policy_basis = "standing://routine@2"' "$SB/docs/warrants/$SB_LIB/authorization.toml" \
    && grep -qF 'policy_basis = "standing://routine@1"' "$SB_AUTH"; then
    st_ok "revision 2 covers what 1 did not" "lib/ refused under @1, covered under @2; $SB_W1 keeps @1"
else
    st_fail "revision 2 covers what 1 did not" "r1 $SB_STATUS_R1, r2 $SB_STATUS: $(st_errors "$SB_OUT_R1$SB_OUT")"
fi

# OBL-006: a covered Warrant never takes a path from work in flight.
SB_O=$(st_warrant "$SB" "" "In flight, signed individually" src/shared.rs)
st_sign "$SB" "$SB_O" >/dev/null
SB_C=$(st_warrant "$SB" "standing://routine@2" "Routine, same file" src/shared.rs)
SB_C_BEFORE=$(st_tree "$SB/docs/warrants/$SB_C")
SB_OUT=$(sb standing apply "$SB_C" 2>&1); SB_STATUS=$?
if [[ -f "$SB/docs/warrants/$SB_O/authorization.toml" ]] && [[ $SB_STATUS -ne 0 ]] \
    && grep -E '^ERROR +standing\.in-flight-owner' <<<"$SB_OUT" | grep -q "$SB_O" \
    && [[ "$(st_tree "$SB/docs/warrants/$SB_C")" == "$SB_C_BEFORE" ]]; then
    st_ok "a path owned by work in flight" "standing.in-flight-owner names $SB_O, nothing written"
else
    st_fail "a path owned by work in flight" "exit $SB_STATUS: $(st_errors "$SB_OUT")"
fi

# OBL-002: after revocation, a Warrant the class would have covered is
# refused; the records made under it stand.
SB_R=$(st_warrant "$SB" "standing://routine@2" "Before the revocation" src/o.rs)
SB_OUT=$(sb standing apply "$SB_R" --dry-run 2>&1); SB_BEFORE_REVOKE=$?
SB_REVOKE=$(st_sign "$SB" standing:routine@2 --revoke); SB_REVOKE_STATUS=$?
SB_CHECK=$(sb check 2>&1)
if [[ $SB_BEFORE_REVOKE -eq 0 ]] && [[ $SB_REVOKE_STATUS -eq 0 ]] && grep -q 'standing.revoked-recorded' <<<"$SB_REVOKE"; then
    st_refused "applied after revocation" "$SB_R" 'standing\.revoked'
else
    st_fail "applied after revocation" "dry run $SB_BEFORE_REVOKE, revoke $SB_REVOKE_STATUS: $(st_errors "$SB_REVOKE")"
fi
if grep -E '^WARN +standing\.revoked' <<<"$SB_CHECK" | grep -q "$SB_LIB" \
    && grep -qE "^PASS +authority\.signed +$SB_LIB" <<<"$SB_CHECK"; then
    st_ok "an earlier record stands (§31)" "standing.revoked is a warning; $SB_LIB still signed"
else
    st_fail "an earlier record stands (§31)" "$(grep -E "standing\.revoked|$SB_LIB" <<<"$SB_CHECK" | head -2 | tr '\n' '|')"
fi
SB_ATT=$(sb attest --all --verify 2>&1); SB_ATT_STATUS=$?
if [[ $SB_ATT_STATUS -eq 0 ]] && grep -q 'attest.checked' <<<"$SB_ATT" \
    && ls "$SB/docs/authority/standing/attestations/"standing-revoke-*.dsse.json >/dev/null 2>&1; then
    st_ok "class acts are attested" "acceptances and the revocation verify"
else
    st_fail "class acts are attested" "exit $SB_ATT_STATUS: $(st_errors "$SB_ATT")"
fi
corpus_gone "$SB"
unset PLANT_ROOT

# ── SC · OBL-005, OBL-007, OBL-006 (resolved owner) ──────────────────────────
PLANT_ROOT=$(st_corpus SC)
# Sourced outside plant.sh, or with a setup that failed inside the command
# substitution, PLANT_ROOT is empty and every `--root` below would name the
# real repository. Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus for SC\n' >&2; exit 9; }
SC=$PLANT_ROOT
sc() { "$WAR" --root "$SC" "$@"; }
: > "$ST_TMP/signs.log"
st_class "$ST_TMP/sc.toml" "routine" 1 '"src/**"'
sc standing propose "$ST_TMP/sc.toml" >/dev/null 2>&1
st_sign "$SC" standing:routine@1 >/dev/null
SC_CLASS_SIGNS=$(st_signs)
: > "$ST_TMP/signs.log"
SC_A=()
for n in 1 2 3; do
    SC_A+=("$(st_warrant "$SC" "standing://routine@1" "Covered change $n" "src/c$n.rs")")
done
for a in "${SC_A[@]}"; do
    sc standing apply "$a" >/dev/null 2>&1
    sc dispatch "$a" STAGE-001 >/dev/null 2>&1
done
SC_PREWORK=$(st_signs)
SC_STARTED=$(grep -l 'dispatch.compiled' "$SC"/docs/warrants/*/journal.jsonl 2>/dev/null | wc -l)
st_resolvable "$SC" "${SC_A[@]}"

# OBL-005, the refusals first: the policy service is refused by name, and a
# resolve request writes nothing.
cat > "$ST_TMP/policy.toml" <<POLICY
schema = "oh.war/resolution-response/v1"
warrant = "${SC_A[0]}"
contract_digest = "$(sc resolve "${SC_A[0]}" 2>/dev/null | sed -n 's/^contract_digest = "\(.*\)"/\1/p')"
resolved_by = "plant-policy"
acting_role = "resolver"
common_outcome = "satisfied"
profile_outcome = "delivered"
meaning = "The policy service resolves a basic mechanical Warrant (§27.3)."
effective_time = "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
POLICY
SC_OUT=$(sc resolve "${SC_A[0]}" --response "$ST_TMP/policy.toml" 2>&1); SC_STATUS=$?
if [[ $SC_STATUS -ne 0 ]] && grep -qE '^ERROR +resolve\.standing-needs-human' <<<"$SC_OUT" \
    && [[ ! -f "$SC/docs/warrants/${SC_A[0]}/resolution.toml" ]]; then
    st_ok "the policy service resolves" "resolve.standing-needs-human, nothing written"
else
    st_fail "the policy service resolves" "exit $SC_STATUS: $(st_errors "$SC_OUT")"
fi
sc resolve "${SC_A[0]}" >/dev/null 2>&1
if [[ ! -f "$SC/docs/warrants/${SC_A[0]}/resolution.toml" ]]; then
    st_ok "war resolve alone" "writes no resolution"
else
    st_fail "war resolve alone" "a resolution.toml appeared"
fi
# A class carrying a resolution term is refused as an unknown term.
st_class "$ST_TMP/resolving.toml" "resolving" 1 '"src/**"' "" 10 'resolution = "policy_service"'
SC_OUT=$(sc standing propose "$ST_TMP/resolving.toml" 2>&1); SC_STATUS=$?
if [[ $SC_STATUS -ne 0 ]] && grep -E '^ERROR +standing\.unknown-term' <<<"$SC_OUT" | grep -q 'resolution' \
    && [[ ! -f "$SC/docs/authority/standing/resolving@1.toml" ]]; then
    st_ok "a class with a resolution term" "standing.unknown-term, nothing written"
else
    st_fail "a class with a resolution term" "exit $SC_STATUS: $(st_errors "$SC_OUT")"
fi
# The positive: one batch, one signature, three covered Warrants resolved by
# a human.
: > "$ST_TMP/signs.log"
SC_OUT=$(PATH="$ST_TMP/bin:$PATH" sc sign --batch "$(IFS=,; echo "${SC_A[*]}")" --ssh-sign --as "Plant Signer" </dev/null 2>&1); SC_STATUS=$?
SC_RESOLVE_SIGNS=$(st_signs)
SC_RESOLVED=$(ls "$SC"/docs/warrants/*/resolution.toml 2>/dev/null | wc -l)
SC_BATCHES=$(ls "$SC"/docs/authority/batches/*.json 2>/dev/null | wc -l)
if [[ $SC_STATUS -eq 0 ]] && grep -q 'batch.recorded' <<<"$SC_OUT" && [[ $SC_RESOLVED -eq 3 ]] && [[ $SC_BATCHES -eq 1 ]] \
    && grep -qF 'resolved_by_ref = "person://Plant Signer"' "$SC/docs/warrants/${SC_A[0]}/resolution.toml"; then
    st_ok "a human resolves them in one batch" "3 resolutions, 1 batch document, by Plant Signer"
else
    st_fail "a human resolves them in one batch" "exit $SC_STATUS, $SC_RESOLVED resolved, $SC_BATCHES batch(es): $(st_errors "$SC_OUT")"
fi

# OBL-007 under B: the same three as ordinary Warrants.
st_commit "$SC" "the covered three resolved"
SC_B=()
for n in 1 2 3; do
    SC_B+=("$(st_warrant "$SC" "" "Ordinary change $n" "src/o$n.rs")")
done
st_commit "$SC" "three ordinary Warrants"
SC_B_STARTED=0
for b in "${SC_B[@]}"; do
    # Captured, not piped: under pipefail a `grep -q` that stops reading
    # early would fail the pipeline and count a refusal as a start.
    SC_OUT=$(sc dispatch "$b" STAGE-001 2>&1)
    grep -q 'dispatch.unauthorized' <<<"$SC_OUT" || SC_B_STARTED=$((SC_B_STARTED + 1))
done
: > "$ST_TMP/signs.log"
PATH="$ST_TMP/bin:$PATH" sc sign --batch "$(IFS=,; echo "${SC_B[*]}")" --ssh-sign --as "Plant Signer" </dev/null >/dev/null 2>&1
st_resolvable "$SC" "${SC_B[@]}"
PATH="$ST_TMP/bin:$PATH" sc sign --batch "$(IFS=,; echo "${SC_B[*]}")" --ssh-sign --as "Plant Signer" </dev/null >/dev/null 2>&1
SC_B_SIGNS=$(st_signs)
SC_B_RESOLVED=0
for b in "${SC_B[@]}"; do [[ -f "$SC/docs/warrants/$b/resolution.toml" ]] && SC_B_RESOLVED=$((SC_B_RESOLVED + 1)); done
if [[ $SC_CLASS_SIGNS -eq 2 ]] && [[ $SC_PREWORK -eq 0 ]] && [[ $SC_STARTED -eq 3 ]] && [[ $SC_RESOLVE_SIGNS -eq 2 ]]; then
    st_ok "A: 2 once, 0 before work, 2 after" "class $SC_CLASS_SIGNS, pre-work $SC_PREWORK ($SC_STARTED started), resolution batch $SC_RESOLVE_SIGNS"
else
    st_fail "A: 2 once, 0 before work, 2 after" "class $SC_CLASS_SIGNS, pre-work $SC_PREWORK ($SC_STARTED started), resolution batch $SC_RESOLVE_SIGNS"
fi
if [[ $SC_B_STARTED -eq 0 ]] && [[ $SC_B_SIGNS -eq 4 ]] && [[ $SC_B_RESOLVED -eq 3 ]]; then
    st_ok "B: 4, and nothing starts first" "0 of 3 dispatched before the authorize batch; 4 signatures for 3"
else
    st_fail "B: 4, and nothing starts first" "$SC_B_STARTED started unsigned, $SC_B_SIGNS signature(s), $SC_B_RESOLVED resolved"
fi

# OBL-006, the positive: a path whose owner is RESOLVED is taken as
# OW-ADR-0021 describes, and the earlier pin reads superseded-by.
SC_T=$(st_warrant "$SC" "standing://routine@1" "Routine, a resolved owner's file" src/c1.rs)
SC_OUT=$(sc standing apply "$SC_T" 2>&1); SC_STATUS=$?
printf '// changed under %s\n' "$SC_T" >> "$SC/src/c1.rs"
SC_CHECK=$(sc check 2>&1)
if [[ $SC_STATUS -eq 0 ]] && grep -q 'standing.recorded' <<<"$SC_OUT" \
    && grep -E 'deliverable\.superseded-by' <<<"$SC_CHECK" | grep -q "${SC_A[0]}" \
    && ! grep -qE "^ERROR +deliverable\.digest-drift .*${SC_A[0]}" <<<"$SC_CHECK"; then
    st_ok "a resolved owner's path is taken" "standing.recorded; ${SC_A[0]}'s pin reads deliverable.superseded-by"
else
    st_fail "a resolved owner's path is taken" "exit $SC_STATUS: $(st_errors "$SC_OUT$(grep -E 'deliverable\.' <<<"$SC_CHECK")")"
fi
corpus_gone "$SC"
unset PLANT_ROOT

ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$ST_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$ST_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
unset SSH_AGENT_PID
command rm -rf "$ST_TMP"
