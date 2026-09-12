#!/usr/bin/env bash
# The drafting agent OW-WAR-0042 names: Claude Code's non-interactive mode,
# a SEPARATE PROCESS over §75.2's seam. The oh.war/draft-request/v1 arrives
# on stdin; an oh.war/draft-proposal/v2 leaves on stdout; nothing else.
#
# The model is given no tools. A drafter that can write files is not
# drafting — `war plan` audits the tree before and after and refuses a
# proposal from a process that touched it (plan.drafter-wrote-files), and
# this wrapper removes the temptation before the audit.
#
# Version-pinned by recording: the wrapper writes the exact `claude --version`
# and the model it asked for beside each run, under $CLAUDE_DRAFTER_LOG when
# set (the OW-WAR-0042 evidence directory), so OBL-001's "named and
# version-pinned" is on record, not asserted.
set -euo pipefail
request=$(cat)
log="${CLAUDE_DRAFTER_LOG:-}"
model="${CLAUDE_DRAFTER_MODEL:-}"
args=(-p --output-format text --no-session-persistence --disallowedTools "Bash,Edit,Write,NotebookEdit,WebFetch,WebSearch,Agent")
[[ -n "$model" ]] && args+=(--model "$model")
system='You are the drafting agent behind `war plan` (OpenWarrant, SAS §74/§75). You will receive an oh.war/draft-request/v1 JSON document on stdin: a vague human request, the namespace, the profile, the assurance level, the existing Warrant aliases and ADR aliases. Answer with EXACTLY ONE JSON document and nothing else — no prose, no code fence — an oh.war/draft-proposal/v2:
{"api_version":"oh.war/draft-proposal/v2","proposed_identity":{"title":"<Imperative title>","profile":"<request profile>","assurance":"<request assurance>"},"operations":[...],"risk_assessment":"<one paragraph>"}
Operations, in this order, each {"op":"create_atom","role":R,"ordinal":N,"path":P,"body":B}:
 - role "intent", ordinal 10, path "10-intent.md": "# Intent" with sections "## Problem", "## Desired Outcome", "## Scope", "## Non-goals" (bullets), "## SAS and Roadmap Traceability".
 - role "basis", ordinal 20, path "20-basis.md": "# Basis" with "## Governing text" (bullets citing SAS sections) and "## Assumptions carried in" (bullets).
 - role "work_order", ordinal 40, path "40-work-order.md": "# Work Order" with "## Deliverables" (numbered list, each naming a file path in backticks), "## Frozen Surfaces", "## Premade Instructions" (bullets), "## Autonomy and Escalation" (a tier T1..T3), "## Rollback".
 - role "milestones", ordinal 45, path "45-milestones.yaml": a flat YAML document: schema: "oh.war/milestones/v1", then milestones: (list of {id: "M1", title, stage_refs: ["STAGE-001"], obligation_refs: ["OBL-001"]}) and stages: (list of {id: "STAGE-001", title, executor_kind: "agent", executor_ref: "agent://claude-code", responsibility_tier: "T1", context_sections: ["40-work-order.md#Premade Instructions", "40-work-order.md#Deliverables"], budget_tokens: 8000}). No nested mappings beyond one level inside list items; every scalar quoted.
 - role "assurance", ordinal 60, path "60-assurance.md": "# Assurance" with "## Acceptance Obligations" containing "### OBL-001 — <title>" blocks, each with bullets "- **scope:** ...", "- **gate:** `gate://software.repo.war-check@1.0.0`", "- **evidence:** ..."; then "## Gate Adequacy" with the line "Required at `<assurance>`.", a "**Adversarial question:** ..." paragraph and "- **outcome:** gap_accepted"; then "## Residual Risk" bullets.
Then exactly one {"op":"add_relation","relation_kind":"roadmap","relation_ref":"roadmap://<NAMESPACE>-PHASE-2/plan"} where <NAMESPACE> is the request namespace.
If the request would require a durable architectural choice (a format, a dependency, a protocol boundary), add {"op":"propose_adr","adr_title":"<title>","body":"<markdown with ## Status, ## Context, ## Decision, ## Consequences>"} — never bury such a choice in the work order.
If the request is too vague to draft without a human answer, answer instead with {"api_version":"oh.war/draft-proposal/v2","proposed_identity":{...},"operations":[],"risk_assessment":"...","blockers":[{"id":"Q1","question":"..."}]}.
Write no files. Run no tools. Output the JSON only.'
started=$(date -u +%Y-%m-%dT%H:%M:%SZ)
version=$(claude --version 2>/dev/null | head -1)
# The system prompt travels as an argument and is therefore visible in `ps`
# on a shared host; it contains nothing secret (the proposal schema), which is
# why that is acceptable here.
# The prompt comes FIRST: --disallowedTools is variadic and would swallow a
# trailing positional. The request itself rides on stdin.
out=$(printf '%s' "$request" | claude -p "Draft a Warrant for the oh.war/draft-request/v1 document on stdin. Output the proposal JSON only." "${args[@]:1}" --append-system-prompt "$system" 2>/tmp/claude-drafter.$$.err) || status=$?
status=${status:-0}
finished=$(date -u +%Y-%m-%dT%H:%M:%SZ)
# Strip a code fence if the model added one despite the instruction.
out=$(printf '%s\n' "$out" | sed -e '/^```/d')
if [[ -n "$log" ]]; then
    mkdir -p "$log"
    printf '%s' "$request" > "$log/request.json"
    printf '%s\n' "$out" > "$log/proposal.raw.json"
    cp /tmp/claude-drafter.$$.err "$log/stderr.txt" 2>/dev/null || true
    printf '{"schema":"oh.war/drafter-run/v1","drafter":"claude-code","version":"%s","model":"%s","argv":%s,"started_at":"%s","finished_at":"%s","exit_status":%s,"tools":"none (all disallowed)"}\n' \
        "$version" "${model:-default}" "$(printf '%s\n' "claude" "${args[@]}" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read().splitlines()))')" "$started" "$finished" "$status" > "$log/run.json"
fi
rm -f /tmp/claude-drafter.$$.err
printf '%s\n' "$out"
exit "$status"
