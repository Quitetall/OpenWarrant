#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# A synthetic program of --n Warrants, --resolved of them resolved, built
# through the real acts (OW-WAR-0120; docs/RETENTION.md).
#
# Every Warrant is made by `war new`. The authorizations are signed by one
# `war sign --batch --ssh-sign`. Every resolved Warrant goes through `war
# evidence record`, `war verify --response` and `war resolve --response`, so
# it carries the records a real one does: an authorization and its signed
# response, a gate-run receipt, a verification, a signed resolution response,
# a resolution, and a journal.
#
# How a resolution is signed (--resolve-with):
#   response  (default) the script drafts the §56.2 response and signs it with
#             the throwaway key through the throwaway agent (`ssh-keygen -Y
#             sign -n oh.war/response`), and `war resolve --response` ingests
#             it: the same ingest `war sign` runs. What this path does not
#             write is the DSSE attestation `war sign` adds beside each
#             resolution. It exists because `war sign` itself was measured
#             at about 9 s per resolution on a 60-Warrant program and 23 s on
#             100: it verifies every recorded signature, many times, by
#             spawning `ssh-keygen` (865 spawns at 60 Warrants, 3,849 at 100).
#             Five hundred of them at 1,000 Warrants is not a bounded run.
#             docs/RETENTION.md records this.
#   sign      `war sign <alias> --ssh-sign`, the full path, attestation
#             included. Use it for small programs. Each resolved Warrant also gets one
# `implementation/` log. The log's size is the median, over this repository's
# resolved Warrants, of their `implementation/` bytes (20-basis A-002).
#
# Everything here is a FIXTURE (20-basis A-003):
#   - "Scale Signer" is a key generated here, held in an ssh-agent started
#     here, which the script checks holds that key and nothing else;
#   - "synth-performer" and "synth-verifier" are names, not actors. The
#     verifier's verdicts are written by this script and examine nothing
#     real;
#   - the gate every obligation cites, `synth.fixture-bytes@1.0.0`, runs
#     `sha256sum --check --quiet synth/SHA256SUMS`, which checks every fixture
#     deliverable's bytes. It is a fixture gate. The program's own `war check`
#     gate is not used: each run of it writes the whole corpus's check output
#     (about 107 KB per receipt at 100 Warrants) and needs a fresh whole-corpus
#     compile, so 500 receipts would be neither bounded nor shaped like a real
#     corpus.
# Nothing it writes leaves --out, and none of it is evidence about any real
# Warrant.
#
# Usage:
#   tools/scale/synth-corpus.sh --n <count> --resolved <count> --out <dir> [--war <binary>]
#                               [--log-bytes <n>] [--resolve-with response|sign]
#
# --out must not exist or be empty; a non-empty one is refused before
# anything is written. --war defaults to `war` on PATH. The counts printed
# at the end come from `war status --json`, not from this script's tally.
# They are also written to <dir>/SYNTH.json, committed in the scratch
# program, so that budget.sh can name the corpus shape it measured.
# Exit 0 when the program checks clean and the counts are what was asked;
# 1 otherwise; 2 on a usage error or a refusal.

set -euo pipefail

usage() {
    sed -n '/^# Usage:/,/^# Exit 0/p' "$0" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

WAR_ARG="war"
N=""
RESOLVED=""
OUT=""
LOG_BYTES=""
RESOLVE_WITH="response"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --war) WAR_ARG="${2:?}"; shift 2 ;;
        --n) N="${2:?}"; shift 2 ;;
        --resolved) RESOLVED="${2:?}"; shift 2 ;;
        --out) OUT="${2:?}"; shift 2 ;;
        --log-bytes) LOG_BYTES="${2:?}"; shift 2 ;;
        --resolve-with) RESOLVE_WITH="${2:?}"; shift 2 ;;
        -h|--help) usage ;;
        *) printf 'synth-corpus.sh: unknown argument %s\n' "$1" >&2; usage ;;
    esac
done
[[ "$N" =~ ^[1-9][0-9]*$ && "$RESOLVED" =~ ^[0-9]+$ && -n "$OUT" ]] || usage
[[ "$RESOLVE_WITH" == response || "$RESOLVE_WITH" == sign ]] || usage
# The program's own first Warrant (from `war init --program`) is one of the
# N and stays a draft, so at most N-1 can be resolved.
(( RESOLVED <= N - 1 )) || { printf 'synth-corpus.sh: --resolved must be at most --n minus 1\n' >&2; exit 2; }

if [[ -e "$OUT" ]]; then
    [[ -d "$OUT" ]] || { printf 'synth-corpus.sh: %s exists and is not a directory; refusing\n' "$OUT" >&2; exit 2; }
    if [[ -n "$(ls -A "$OUT")" ]]; then
        printf 'synth-corpus.sh: %s is not empty; refusing, and nothing was written\n' "$OUT" >&2
        exit 2
    fi
fi

WAR=$(command -v -- "$WAR_ARG" 2>/dev/null || true)
[[ -n "$WAR" && -x "$WAR" ]] || { printf 'synth-corpus.sh: no war binary at %s\n' "$WAR_ARG" >&2; exit 2; }
WAR=$(cd "$(dirname "$WAR")" && printf '%s/%s' "$PWD" "$(basename "$WAR")")

SCRIPT_REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
if [[ -z "$LOG_BYTES" ]]; then
    LOG_BYTES=$(python3 - "$SCRIPT_REPO/docs/warrants" <<'PY'
import os, statistics, sys
root = sys.argv[1]
sizes = []
for w in sorted(os.listdir(root)) if os.path.isdir(root) else []:
    d = os.path.join(root, w)
    if not os.path.isfile(os.path.join(d, "resolution.toml")):
        continue
    total = 0
    for r, _, fs in os.walk(os.path.join(d, "implementation")):
        total += sum(os.path.getsize(os.path.join(r, f)) for f in fs)
    sizes.append(total)
print(int(statistics.median(sizes)) if sizes else 0)
PY
)
fi
[[ "$LOG_BYTES" =~ ^[0-9]+$ ]] || { printf 'synth-corpus.sh: --log-bytes must be a number\n' >&2; exit 2; }

export OPENWARRANT_NO_PROJECTS=1
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
TMP=$(mktemp -d)

if [[ -n "${SSH_AUTH_SOCK+x}" ]]; then OLD_SOCK_SET=1; OLD_SOCK="$SSH_AUTH_SOCK"; else OLD_SOCK_SET=0; OLD_SOCK=""; fi
AGENT_PID=""
cleanup() {
    if [[ -n "$AGENT_PID" ]]; then kill "$AGENT_PID" 2>/dev/null || true; fi
    if [[ "$OLD_SOCK_SET" == 1 ]]; then export SSH_AUTH_SOCK="$OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
    rm -rf "$TMP"
}
trap cleanup EXIT

T0=$(date +%s)
say() { printf '[%4ss] %s\n' $(( $(date +%s) - T0 )) "$*" >&2; }
die() { printf 'synth-corpus.sh: %s\n' "$*" >&2; exit 1; }
quiet() { # run, keep the output only if it fails
    local log="$TMP/last.log"
    if ! "$@" >"$log" 2>&1 </dev/null; then
        printf 'synth-corpus.sh: failed: %s\n' "$*" >&2
        grep -E '^(ERROR|FAIL|UNKNOWN)' "$log" | head -5 >&2 || tail -5 "$log" >&2
        exit 1
    fi
}

# `war` by name resolves to the binary under test: gates run `war` from PATH.
mkdir -p "$TMP/bin"
ln -s "$WAR" "$TMP/bin/war"
export PATH="$TMP/bin:$PATH"
GIT=(git -C "$OUT" -c user.email=synth@invalid -c user.name=synth -c commit.gpgsign=false)
commit() { "${GIT[@]}" add -A >/dev/null; "${GIT[@]}" commit -qm "$1" >/dev/null || true; }

# --- the program, and a throwaway signer ----------------------------------------
cd "$OUT"
git init -q .
quiet war init --program "Scale" --namespace SC
ssh-keygen -q -t ed25519 -N "" -C synth -f "$TMP/id_synth"
KEY_FP=$(ssh-keygen -lf "$TMP/id_synth.pub" | awk '{print $2}')
printf 'synth namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$TMP/id_synth.pub")" > docs/authority/allowed_signers
cat > docs/authority/roles.toml <<'ROLES'
[[assignment]]
actor = "Scale Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "tools/scale/synth-corpus.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "A throwaway identity in a synthetic fixture program. It signs with a key generated for this run only."
ssh_principal = "synth"
ROLES
unset SSH_AUTH_SOCK SSH_AGENT_PID
eval "$(ssh-agent -s -a "$TMP/agent.sock")" >/dev/null
AGENT_PID="$SSH_AGENT_PID"
ssh-add -q "$TMP/id_synth" 2>/dev/null
KEYS=$(ssh-add -l 2>/dev/null || true)
if [[ "$(printf '%s\n' "$KEYS" | grep -c .)" != 1 ]] || ! grep -qF -- "$KEY_FP" <<<"$KEYS"; then
    die "the throwaway agent holds more or other than its own key; refusing"
fi

# The fixture gate: every fixture deliverable's bytes against synth/SHA256SUMS.
cat > docs/gates/synth.fixture-bytes@1.0.0.yaml <<'GATEDEF'
# A FIXTURE gate of tools/scale/synth-corpus.sh. It lives only in this
# synthetic program and checks only the synthetic deliverables.
gate_id: "synth.fixture-bytes"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "artifact://coreutils/sha256sum"
output_schema_ref: "schema://sha256sum-check/v1"
provenance: "local_candidate"
input_kinds: ["fixture-files"]
argv: ["sha256sum", "--check", "--quiet", "synth/SHA256SUMS"]
mutating: "false"
timeout_secs: "120"
fault_model: ["fixture-missing", "fixture-bytes-changed"]
known_blind_spots: ["Checks the bytes of the files synth/SHA256SUMS lists, and nothing about any other file.", "A fixture: nothing real is examined."]
qualification_qualifier: "tools/scale/synth-corpus.sh, for a synthetic program only"
qualification_digest: ""
qualification_positive_controls: ["A changed or deleted fixture file makes sha256sum --check exit 1."]
qualification_negative_controls: ["The unmodified fixture files make it exit 0."]
qualification_mutation_classes: ["deletion", "substitution"]
qualification_environments: ["linux-x86_64, GNU coreutils"]
qualification_limitations: ["Not qualified against anything but the synthetic fixtures it was written for."]
detection_results:
  - fault_class: "fixture-missing"
    mutation: "deleted a listed fixture file"
    detected: "true"
  - fault_class: "fixture-bytes-changed"
    mutation: "appended a byte to a listed fixture file"
    detected: "true"
GATEDEF
mkdir -p synth
: > synth/SHA256SUMS
GATE="gate://synth.fixture-bytes@1.0.0"

quiet war sas propose 0.1.0
quiet war sign 0.1.0 --ssh-sign
commit "program, signer, SAS 0.1.0"
say "program SC, SAS accepted"

# --- N-1 Warrants through `war new`, with real atoms ----------------------------
atom_body() { # <file> <body>: keep the frontmatter `war new` wrote, replace the rest
    python3 - "$1" "$2" <<'PY'
import sys
p, body = sys.argv[1], sys.argv[2]
s = open(p, encoding="utf-8").read()
i = s.index("\n---\n", 4) + 5
open(p, "w", encoding="utf-8").write(s[:i] + "\n" + body.rstrip("\n") + "\n")
PY
}

author() { # <alias>
    local a="$1" wd="docs/warrants/$1" dg
    atom_body "$wd/atoms/10-intent.md" "# Intent

A synthetic fixture Warrant of tools/scale/synth-corpus.sh: deliver
\`synth/$a.txt\`. It exists to give the scale budget a corpus to measure.

## Out of scope

Everything else."
    atom_body "$wd/atoms/20-basis.md" "# Basis

## Governing text

- \`docs/sas/Scale_SAS.md\` at revision 0.1.0.

## Unknowns

- None. This is a fixture."
    atom_body "$wd/atoms/40-work-order.md" "# Work Order

## Deliverables

1. \`synth/$a.txt\`.

## Frozen Surfaces

Every other file.

## Rollback

Delete \`synth/$a.txt\`."
    cat > "$wd/atoms/45-milestones.yaml" <<'YAML'
schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "The fixture file"
    stage_refs: ["STAGE-001"]
    obligation_refs: ["OBL-001"]

stages:
  - id: "STAGE-001"
    title: "Write the fixture file"
    executor_kind: "agent"
    responsibility_tier: "T2"
YAML
    atom_body "$wd/atoms/60-assurance.md" "# Assurance

## Acceptance Obligations

### OBL-001 — the fixture file exists with the declared bytes
- **scope:** \`synth/$a.txt\` in this synthetic program only.
- **gate:** \`$GATE\`
- **evidence:** the file's digest, checked by \`sha256sum --check\` against \`synth/SHA256SUMS\`.

## Gate Adequacy

Required at \`basic\`.

**Adversarial question:** can the obligation read established while the
file is absent? No: requirement 3 recomputes the digest from the bytes.

- **outcome:** no_counterexample"
    printf 'schema = "oh.war/rationale/v1"\n' > "$wd/rationale.toml"
    mkdir -p synth
    printf 'synthetic fixture deliverable of %s\n' "$a" > "synth/$a.txt"
    dg=$(sha256sum "synth/$a.txt" | cut -d' ' -f1)
    printf '%s  synth/%s.txt\n' "$dg" "$a" >> synth/SHA256SUMS
    cat > "$wd/deliverables.toml" <<TOML
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "The fixture file"
kind = "file"
target_ref = "synth/$a.txt"
required = true
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001"]

[deliverable.provenance]
producer = "synth-performer"
producing_attempt = "$a/attempt-1"
contract_digest = "unrecorded"
tool_or_runtime_identity = "tools/scale/synth-corpus.sh"
creation_method = "generated"
content_digest = "sha256:$dg"
media_type = "text/plain"
classification = "internal"
retention = "repository-lifetime"
source_holder = "git"
TOML
}

ALIASES=()
for k in $(seq 2 "$N"); do
    a=$(printf 'SC-WAR-%04d' "$k")
    quiet war new "Synthetic Warrant $k"
    [[ -f "docs/warrants/$a/manifest.toml" ]] || die "war new did not create $a"
    author "$a"
    ALIASES+=("$a")
    if (( k % 100 == 0 )); then say "$k Warrants"; fi
done
quiet war compile
commit "N-1 synthetic Warrants"
say "$((N - 1)) Warrants drafted"

# The first RESOLVED are resolved; of the rest, half are authorized and left
# in flight, and the others stay drafts beside the program's own first one.
REST=$(( N - 1 - RESOLVED ))
AUTHORIZE=$(( RESOLVED + REST / 2 ))
if (( AUTHORIZE > 0 )); then
    TARGETS=$(IFS=,; echo "${ALIASES[*]:0:$AUTHORIZE}")
    quiet war sign --batch "$TARGETS" --ssh-sign --as "Scale Signer"
    quiet war compile
    commit "$AUTHORIZE authorizations, one batch"
fi
say "$AUTHORIZE authorized (one batch signature)"

# --- resolve: evidence, an independent-shaped verdict, a signed resolution -------
resolve_by_response() { # <alias>: draft the §56.2 response, sign it, ingest it
    local a="$1" r="docs/authority/responses/$1.resolution.response.toml" cd title
    cd=$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["revision"]["contract_digest"])' \
        "docs/warrants/$a/authorization.toml")
    title=$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["title"])' "docs/warrants/$a/manifest.toml")
    cat > "$r" <<TOML
schema = "oh.war/resolution-response/v1"
warrant = "$a"
contract_digest = "$cd"
resolved_by = "Scale Signer"
acting_role = "resolver"
common_outcome = "satisfied"
profile_outcome = "delivered"
meaning = "FIXTURE. Resolving $a ($title) satisfied at revision 1, in a synthetic program built by tools/scale/synth-corpus.sh. Drafted and signed by that script with a throwaway key; no human read it, and it is evidence about nothing real."
effective_time = "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
TOML
    ssh-keygen -Y sign -q -f "$TMP/id_synth.pub" -n oh.war/response "$r" 2>/dev/null \
        || die "ssh-keygen could not sign $r with the throwaway agent"
    quiet war resolve "$a" --response "$r"
    [[ -f "docs/warrants/$a/resolution.toml" ]] || die "war resolve --response recorded nothing for $a"
}
for i in $(seq 0 $(( RESOLVED - 1 ))); do
    a="${ALIASES[$i]}"
    quiet war evidence record "$a"
    cat > "$TMP/verdict.toml" <<TOML
schema = "oh.war/verification-response/v1"
warrant = "$a"

[[verifications]]
obligation = "OBL-001"
disposition = "established"
evidence = "FIXTURE (tools/scale/synth-corpus.sh): synth/$a.txt and the gate-run receipt under docs/warrants/$a/gate-runs/. A synthetic verdict; it examined nothing real."
performer = "synth-performer"

[verifications.verifier]
actor = "synth-verifier"
kind = "agent"
model = "synthetic-fixture"

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
TOML
    quiet war verify "$a" --response "$TMP/verdict.toml"
    if [[ "$RESOLVE_WITH" == sign ]]; then
        quiet war sign "$a" --ssh-sign --as "Scale Signer"
    else
        resolve_by_response "$a"
    fi
    mkdir -p "docs/warrants/$a/implementation"
    python3 -c 'import sys; n=int(sys.argv[2]); line=b"synthetic implementation log line of tools/scale/synth-corpus.sh\n"; open(sys.argv[1],"wb").write((line*(n//len(line)+1))[:n])' \
        "docs/warrants/$a/implementation/synth-run.log" "$LOG_BYTES"
    if (( (i + 1) % 50 == 0 )); then commit "resolved through $a"; say "$((i + 1)) resolved"; fi
done
quiet war compile
commit "resolutions and projections"
say "$RESOLVED resolved"

# --- what the tool says was reached ---------------------------------------------
CHECK_RC=0
war check >"$TMP/check.log" 2>&1 </dev/null || CHECK_RC=$?
war --json status >"$TMP/status.json" 2>/dev/null </dev/null || true
SYNTH_RC=0
python3 - "$TMP/status.json" "$N" "$RESOLVED" "$CHECK_RC" "$LOG_BYTES" "$(( $(date +%s) - T0 ))" "$WAR" "$RESOLVE_WITH" "$AUTHORIZE" "$OUT/SYNTH.json" <<'PY' || SYNTH_RC=$?
import collections, json, sys
path, n, resolved, check_rc, log_bytes, secs, war, resolve_with, authorized, marker = sys.argv[1:]
try:
    ws = json.load(open(path))["result"]["warrants"]
except Exception as e:
    print(json.dumps({"schema": "openwarrant.scale/synth-counts/v1", "error": f"war status --json unreadable: {e}"}))
    sys.exit(1)
phases = collections.Counter(w["state"]["phase"] for w in ws)
out = {
    "schema": "openwarrant.scale/synth-counts/v1",
    "counted_by": "war status --json",
    "warrants": len(ws),
    "by_phase": dict(sorted(phases.items())),
    "resolved": phases.get("resolved", 0),
    "asked": {"n": int(n), "resolved": int(resolved)},
    "implementation_log_bytes": int(log_bytes),
    "war_check_exit": int(check_rc),
    "war": war,
    "seconds": int(secs),
    "generator": "tools/scale/synth-corpus.sh",
    "resolve_with": resolve_with,
    "authorized_by_batch": int(authorized),
    "gate": "gate://synth.fixture-bytes@1.0.0 (fixture: sha256sum --check synth/SHA256SUMS)",
    "fixture": "every record is synthetic; the signer, performer and verifier are throwaway names",
}
print(json.dumps(out, sort_keys=True))
with open(marker, "w", encoding="utf-8") as f:
    json.dump(out, f, indent=2, sort_keys=True)
    f.write("\n")
ok = out["warrants"] == int(n) and out["resolved"] == int(resolved) and int(check_rc) == 0
sys.exit(0 if ok else 1)
PY
commit "SYNTH.json: what war status --json counted"
exit "$SYNTH_RC"
