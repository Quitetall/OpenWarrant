#!/usr/bin/env bash
set -o pipefail
source conformance/lib.sh || exit 9
WAR=${PROBE_WAR:-./target/debug/war}
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
# st_snap <dir>: one `<sha256>  ./<path>` line per file, so a changed tree can
# say WHICH paths moved (t-0f24), not only that its digest did.
st_snap() { (cd "$1" && find . -type f -print0 | sort -z | xargs -0 sha256sum); }
# st_moved <before> <after>: the paths whose line differs, space-separated.
st_moved() { diff <(printf '%s\n' "$1") <(printf '%s\n' "$2") | sed -n 's/^[<>] [0-9a-f]*  //p' | sort -u | tr '\n' ' '; }
# st_line_has <text> <ERE> <fixed>: some line matches ERE and contains fixed.
#
# Not `grep -E … | grep -q …`: under `set -o pipefail` the second grep exits
# at its first match, and when the first has more than one pipe buffer's worth
# of matching lines to write (the `**` refusal names 21 authority paths, 6 KB)
# its next write takes SIGPIPE and the pipeline reads false. That is what
# failed 'propose refuses **' once in three concurrent batteries (t-0f24):
# 10 in 2000 evaluations of that pipeline over the same bytes, unloaded.
st_line_has() {
    local lines
    lines=$(grep -E -- "$2" <<<"$1") || return 1
    grep -qF -- "$3" <<<"$lines"
}
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

# st_resolvable <root> <alias>...: record each gate, then simulate independent
# verification of those exact receipts and subjects.
st_resolvable() {
    local root=$1 a
    shift
    for a in "$@"; do
        "$WAR" --root "$root" pins --refresh --alias "$a" >/dev/null 2>&1
    done
    "$WAR" --root "$root" compile >/dev/null 2>&1
    st_commit "$root" "prepared"
    # One gate run per Warrant, each over projections brought up to date:
    # a run records evidence the corpus status projects, and the next run's
    # `war check --generated` would otherwise see the last one's drift.
    for a in "$@"; do
        PATH="$ST_TMP/bin:$PATH" "$WAR" --root "$root" evidence record "$a" >/dev/null 2>&1
        "$WAR" --root "$root" compile >/dev/null 2>&1
    done
    st_commit "$root" "evidence"
    for a in "$@"; do
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
        "$WAR" --root "$root" verify "$a" --performer claude --bundle --json > "$ST_TMP/request-$a.json"
        python3 "$REPO_ROOT/conformance/fixtures/verifier/with-subject.py" "$ST_TMP/request-$a.json" "$ST_TMP/verification-$a.toml" > "$ST_TMP/bound-$a.toml"
        "$WAR" --root "$root" verify "$a" --response "$ST_TMP/bound-$a.toml" >/dev/null 2>&1
    done
    "$WAR" --root "$root" compile >/dev/null 2>&1
    st_commit "$root" "verified"
}

st_sign() { # root target [flags] — one act, ssh-signed as the plant signer
    local root=$1 target=$2
    shift 2
    PATH="$ST_TMP/bin:$PATH" "$WAR" --root "$root" sign "$target" --ssh-sign --as "Plant Signer" "$@" </dev/null 2>&1
}
st_sign_as() { # root target actor — one act, ssh-signed as someone else
    PATH="$ST_TMP/bin:$PATH" "$WAR" --root "$1" sign "$2" --ssh-sign --as "$3" </dev/null 2>&1
}


trap '[[ -n "${SSH_AGENT_PID:-}" ]] && ssh-agent -k >/dev/null 2>&1' EXIT
PLANT_ROOT=$(st_corpus SR)
[[ -d "$PLANT_ROOT/.git" ]] || exit 9
SR=$PLANT_ROOT
sr() { "$WAR" --root "$SR" "$@"; }
st_class "$ST_TMP/one.toml" "race-one" 1 '"src/**"' "" 1
sr standing propose "$ST_TMP/one.toml" > "$PROOF/propose.log" 2>&1 || exit 9
st_sign "$SR" standing:race-one@1 > "$PROOF/sign.log" 2>&1 || exit 9
A=$(st_warrant "$SR" "standing://race-one@1" "First concurrent change" src/race-a.rs)
B=$(st_warrant "$SR" "standing://race-one@1" "Second concurrent change" src/race-b.rs)
st_commit "$SR" "two distinct scope contracts"
mkdir -p "$PROOF/observed"
printf '%s\n' "$SR" > "$PROOF/fixture-root.txt"
sr standing apply "$A" --class standing://race-one@01 --json > "$PROOF/observed/a.json" 2>&1; echo $? > "$PROOF/observed/a.exit"
sr standing apply "$B" --class standing://race-one@1 --json > "$PROOF/observed/b.json" 2>&1; echo $? > "$PROOF/observed/b.exit"
sr standing show race-one --json > "$PROOF/observed/class.json"
printf '%s\n%s\n' "$A" "$B" > "$PROOF/aliases.txt"
python3 - "$PROOF" "$SR" "$A" "$B" <<'CHECK'
import pathlib,json,sys
p=pathlib.Path(sys.argv[1]);root=pathlib.Path(sys.argv[2]);rows=[]
for key,alias in zip(['a','b'],sys.argv[3:]):
 text=(p/'observed'/f'{key}.json').read_text()
 rows.append({'alias':alias,'exit':int((p/'observed'/f'{key}.exit').read_text()),'authorization_exists':(root/'docs/warrants'/alias/'authorization.toml').exists(),'recorded':'standing.recorded' in text,'exhausted':'standing.exhausted' in text})
obs={'limit':1,'requests':rows,'recorded':sum(x['authorization_exists'] for x in rows)}
(p/'observation.json').write_text(json.dumps(obs,indent=2)+'\n');print(json.dumps(obs))
assert obs['recorded']==1, 'standing class limit exceeded before refusal'
CHECK
