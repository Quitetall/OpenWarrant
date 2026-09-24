#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# claude-performer.sh — a `war perform` performer that hands the Dispatch to
# `claude -p` and passes back its Stage Submission, or nothing (OW-WAR-0131).
#
# Configure it once, in openwarrant.toml (see docs/PERFORM.md):
#
#     [perform]
#     performer_argv = ["tools/performer/claude-performer.sh"]
#
# The contract with `war perform` is the one every performer has: the Dispatch
# (oh.war/stage-dispatch/v1) arrives on stdin, and stdout is either one
# oh.war/stage-submission/v1 or empty. `war perform` still puts whatever comes
# back through `war submit`'s refusals; the checks here are so a malformed or
# self-completing answer is refused at the adapter too, with a reason on
# stderr, rather than printed.
#
# What this adapter does NOT do:
#
# - write a submission `claude` did not produce. Stdout is the model's answer,
#   byte for byte, or nothing at all.
# - grant `claude` more than ALLOWED_TOOLS below. It runs headless (`-p`), so a
#   tool outside the list has no approval path and is refused (§55.2). The list
#   includes Bash by the owner's decision of 2026-09-24: a performer that
#   cannot run the build and the plants cannot check its own work. The cost is
#   named, not hidden: a shell can start a process outside the performer's
#   process group (`setsid`, a daemon), which `war perform` cannot then cancel
#   (docs/PERFORM.md, "Not covered"). The conformance plant pins the list, so
#   any further change is visible.
# - bound itself. `war perform` bounds it: the performer's wall time and a
#   cancellation kill this script and `claude` together, as one process group.
#
# Environment:
#
#   CLAUDE_PERFORMER_LOG  a file to which the raw answer (and claude's stderr)
#                         is appended, whether or not it was accepted.
#
# Exit status: 0 with the submission on stdout; non-zero with empty stdout and
# `claude-performer: refused: <reason>: <detail>` on stderr otherwise.

set -uo pipefail

ALLOWED_TOOLS="Read,Glob,Grep,Edit,Write,Bash"

say() { printf 'claude-performer: %s\n' "$*" >&2; }
refuse() {
    say "refused: $1: $2"
    exit 3
}

command -v python3 > /dev/null 2>&1 || refuse no-python3 "python3 is needed to read the Dispatch"
command -v claude > /dev/null 2>&1 || refuse no-claude "no \`claude\` on PATH"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/claude-performer.XXXXXX") || refuse no-scratch "mktemp failed"
trap 'rm -rf "$tmp"' EXIT

cat > "$tmp/dispatch.json"
dispatch_id=$(python3 - "$tmp/dispatch.json" <<'PY' 2> /dev/null
import json, sys
d = json.load(open(sys.argv[1], encoding="utf-8"))
i = d.get("dispatch_id") if isinstance(d, dict) else None
if not isinstance(i, str) or not i:
    sys.exit(1)
print(i)
PY
) || refuse not-a-dispatch "stdin is not a Stage Dispatch naming a dispatch_id"

# The prompt: a fixed instruction, then the Dispatch verbatim.
{
    cat << 'EOF'
You are the performer of one stage of an authorized OpenWarrant Warrant. The
Stage Dispatch below (oh.war/stage-dispatch/v1) is your whole brief: do the
work it describes, in this repository, and nothing outside it.

When you are done, answer with exactly one JSON object and nothing else — no
prose, no code fence. It is an oh.war/stage-submission/v1:

  "schema": "oh.war/stage-submission/v1",
  "dispatch_id", "attempt_id", "contract_digest", "stage_id": copied from the
      Dispatch,
  "claims": [{"id": "C-001", "statement": "..."}] — what you did, stated so
      someone else can check it,
  "artifact_refs": [...] — the files you changed,
  "blockers": [...] — anything that stopped you,
  "requested_next_action": one of "continue", "verify", "block", "amend",
      "cancel".

You cannot complete, verify or resolve your own work (§51.2): never ask for
resolution. If you could not do the work, say so in "blockers" and ask to
"block".

The Stage Dispatch:
EOF
    cat "$tmp/dispatch.json"
} > "$tmp/prompt"

claude -p --output-format text --allowedTools "$ALLOWED_TOOLS" \
    < "$tmp/prompt" > "$tmp/answer" 2> "$tmp/stderr"
rc=$?

if [[ -n "${CLAUDE_PERFORMER_LOG:-}" ]]; then
    {
        printf '=== %s dispatch %s: claude exited %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$dispatch_id" "$rc"
        cat "$tmp/answer"
        printf '\n--- stderr\n'
        cat "$tmp/stderr"
        printf '\n'
    } >> "$CLAUDE_PERFORMER_LOG"
fi

if [[ $rc -ne 0 ]]; then
    tail -n 5 "$tmp/stderr" >&2
    refuse claude-failed "claude exited $rc; its answer, if any, is not passed on"
fi

reason=$(python3 - "$tmp/answer" "$dispatch_id" <<'PY'
import json, sys
path, want = sys.argv[1], sys.argv[2]
try:
    value = json.loads(open(path, encoding="utf-8").read())
except Exception:
    print("not-json"); sys.exit(1)
if not isinstance(value, dict) or value.get("schema") != "oh.war/stage-submission/v1":
    print("not-a-submission"); sys.exit(1)
if value.get("dispatch_id") != want:
    print("dispatch-mismatch"); sys.exit(1)
# §51.2's five, and nothing else: anything outside them is a request to be
# completed, which a performer may not make.
if value.get("requested_next_action") not in ("continue", "verify", "block", "amend", "cancel"):
    print("self-completion"); sys.exit(1)
PY
) || refuse "${reason:-unreadable}" "claude's answer is not a Stage Submission for dispatch $dispatch_id that a performer may send"

cat "$tmp/answer"
