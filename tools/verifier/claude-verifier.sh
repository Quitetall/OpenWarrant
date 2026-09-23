#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The blind verifier `war verify --run` hands a bundle to (SAS §46, §75.2;
# OW-WAR-0117).
#
# A SEPARATE PROCESS, a separate context, and no tools:
#   --tools ""            no built-in tool at all (an allowlist of nothing, so
#                         a tool a later Claude Code adds is not available
#                         either — a denylist would let it through)
#   --strict-mcp-config   no MCP server: this machine's servers can write files
#   --restricted          user, project and local settings files ignored
#   --disallowedTools     the named tools refused as well, in case an older
#                         Claude Code reads `--tools ""` differently
#   --no-session-persistence
#   an empty temporary working directory, so no project CLAUDE.md and no
#   project auto-memory (which holds a performer's own notes) is discovered
#
# It reads the bundle `war` compiled, on stdin, and nothing else. The bundle
# carries the contract atoms, the deliverables' bytes, the plants naming the
# Warrant, the gate runs and prior verifications — and nothing the performer
# said about them.
#
# The model chooses, per obligation, a disposition and the evidence for it —
# nothing else. Who the verifier is, and the independence it has, are written
# by THIS SCRIPT from how it is built, never by the model: a model cannot
# grant itself independence by saying so.
#
# Usage (as `[verify] verifier_argv`): claude-verifier.sh <bundle.json>
# Environment:
#   CLAUDE_VERIFIER_MODEL   the verifier's model (default claude-sonnet-5)
#   CLAUDE_PERFORMER_MODEL  the performer's model. `distinct_model_required`
#                           is claimed only when this is set and differs.
#   CLAUDE_VERIFIER_LOG     a directory for the run record: version, model,
#                           times, the raw answer
#   CLAUDE_BIN              the claude executable (default: claude on PATH)
set -euo pipefail
bundle="${1:?usage: claude-verifier.sh <bundle.json>}"
bundle=$(realpath "$bundle")
model="${CLAUDE_VERIFIER_MODEL:-claude-sonnet-5}"
performer_model="${CLAUDE_PERFORMER_MODEL:-}"
log="${CLAUDE_VERIFIER_LOG:-}"
claude="${CLAUDE_BIN:-claude}"
system='You are an independent verifier (OpenWarrant, SAS §46). You receive one oh.war/verification-bundle/v1 JSON document on stdin: the Warrant'"'"'s authorized contract (its atoms), the bytes of its deliverables, the conformance plants that name it, its recorded gate runs, and any prior verifications. You did not do this work and you have not seen anyone'"'"'s account of it.

For EACH obligation in request.obligations, decide from the bundle alone:
- "established": the evidence in the bundle shows the obligation'"'"'s claim holds, within its stated scope, and shows a refusal where the obligation asks for one.
- "refuted": the bundle shows the claim does not hold.
- "not_established": the bundle does not carry enough to decide either way. This is the honest answer whenever the evidence is absent, truncated past what you need, or only asserted.

Never establish an obligation because it is probably true, because the code looks plausible, or because a deliverable exists. A claim is established by what the bundle shows, bounded by the obligation'"'"'s scope.

Answer with EXACTLY ONE JSON document and nothing else — no prose, no code fence:
{"verdicts":[{"obligation":"OBL-001","disposition":"established|refuted|not_established","evidence":"<one paragraph naming the specific bundle items — file, test name, plant line, gate run id — that decide it>"}]}'
argv=(-p "Verify the oh.war/verification-bundle/v1 on stdin. Output the verdicts JSON only."
    --output-format text --no-session-persistence --model "$model"
    --tools "" --strict-mcp-config --restricted
    --disallowedTools "Bash,Edit,Write,NotebookEdit,WebFetch,WebSearch,Agent,Read,Glob,Grep,Task"
    --append-system-prompt "$system")
work=$(mktemp -d)
err=$(mktemp)
answer=$(mktemp)
trap 'rm -rf "$work" "$err" "$answer"' EXIT
started=$(date -u +%Y-%m-%dT%H:%M:%SZ)
version=$("$claude" --version 2>/dev/null | head -1 || true)
status=0
(cd "$work" && "$claude" "${argv[@]}" < "$bundle") > "$answer" 2> "$err" || status=$?
finished=$(date -u +%Y-%m-%dT%H:%M:%SZ)
if [[ -n "$log" ]]; then
    mkdir -p "$log"
    cp "$answer" "$log/verdicts.raw.json"
    cp "$err" "$log/stderr.txt"
    python3 - "$version" "$model" "$started" "$finished" "$status" > "$log/run.json" <<'PY'
import json, sys
v, m, s, f, st = sys.argv[1:6]
print(json.dumps({"schema": "oh.war/verifier-run/v1", "verifier": "claude-code", "version": v,
                  "model": m, "started_at": s, "finished_at": f, "exit_status": int(st),
                  "tools": "none: --tools '' --strict-mcp-config --restricted, empty working directory"}))
PY
fi
if [[ "$status" -ne 0 ]]; then
    echo "claude exited $status: $(head -c 400 "$err")" >&2
    exit "$status"
fi
# The response: the model's verdicts, the script's identity and independence.
python3 - "$bundle" "$model" "$performer_model" "$answer" <<'PY'
import json, re, sys
bundle = json.load(open(sys.argv[1]))
model, performer_model = sys.argv[2], sys.argv[3]
text = open(sys.argv[4]).read().strip()
text = re.sub(r"^```[a-z]*\n|\n```$", "", text)
try:
    verdicts = {v["obligation"]: v for v in json.loads(text)["verdicts"]
                if isinstance(v, dict) and "obligation" in v}
except Exception:
    # Unknown is not pass (Law 15): an answer that does not parse settles
    # nothing, so every obligation reads not_established.
    verdicts = {}
allowed = {"established", "refuted", "not_established"}
def q(s):
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ") + '"'
distinct = bool(performer_model) and performer_model != model
print('schema = "oh.war/verification-response/v1"')
print("warrant = %s" % q(bundle["warrant"]))
for o in bundle["request"]["obligations"]:
    v = verdicts.get(o["id"], {})
    d = v.get("disposition") if v.get("disposition") in allowed else "not_established"
    ev = v.get("evidence") if d == v.get("disposition") and v.get("evidence") else (
        "the verifier returned no usable verdict for this obligation")
    print("")
    print("[[verifications]]")
    print("obligation = %s" % q(o["id"]))
    print("disposition = %s" % q(d))
    print("evidence = %s" % q(ev))
    print("performer = %s" % q(bundle["request"]["performer"]))
    print("[verifications.verifier]")
    print("actor = %s" % q("claude-verifier (" + model + ")"))
    print('kind = "agent"')
    print("[verifications.verifier.independence]")
    # By construction of this script, never by the model's word:
    print("performer_transcript_blind = true")   # the bundle carries no transcript
    print("performer_rationale_blind = true")    # nor rationale or journal; no memory is loaded
    print("separate_writable_workspace = true")  # an empty temporary directory, and no tool to write
    print("cannot_modify_subject_artifacts = true")  # no tools, no MCP
    print("cannot_modify_gate_definition = true")    # no tools, no MCP
    print("cannot_modify_gate_fixtures = true")      # no tools, no MCP
    print("separate_context_compilation = true")     # `war` compiled the bundle
    print("distinct_model_required = %s" % ("true" if distinct else "false"))  # checked, not assumed
    print("distinct_human_required = false")     # no human verified this
PY
