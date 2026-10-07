#!/usr/bin/env bash
# PreToolUse hook (Edit|Write|MultiEdit|NotebookEdit): refuse an edit to a file
# a RESOLVED Warrant pins and no later authorized Warrant governs, or to
# anything under a generated/ directory.
#
# The pin list is `war pins --resolved-only --json`, so the hook can never
# disagree with `war check`: both read the same records. A pin a LATER
# authorized Warrant's recorded set governs is `historical` (OW-ADR-0021) and
# the edit goes through under that Warrant's authority; a current pin changes
# only through the correction act (`war correct`), which a human signs. A pin
# with no `historical` field at all comes from a binary older than the rule and
# is treated as current — fail-closed.
#
# The checkout's own binary is preferred over the one on PATH, as
# `stop-check.sh` does: the rule that decides an edit should be the rule the
# tree was built with, not whatever an older install still says.
#
# Fail-open with a note when neither is available: a missing guard is said,
# not silently skipped, and `war check` still catches the drift later.
set -uo pipefail

input=$(cat)
file=$(printf '%s' "$input" | python3 -c 'import sys, json
try:
    print(json.load(sys.stdin).get("tool_input", {}).get("file_path", ""))
except Exception:
    print("")')
cwd=$(printf '%s' "$input" | python3 -c 'import sys, json
try:
    print(json.load(sys.stdin).get("cwd", ""))
except Exception:
    print("")')
[[ -z "$file" ]] && exit 0
if [[ -n "$cwd" ]] && ! cd "$cwd" 2>/dev/null; then
    printf 'openwarrant guard-pins: cannot enter %s; the pin guard did not run\n' "$cwd" >&2
    exit 0
fi

deny() {
    printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":%s}}\n' \
        "$(printf '%s' "$1" | python3 -c 'import sys, json; print(json.dumps(sys.stdin.read()))')"
    exit 0
}

# Relative to the repository root when the path is absolute and inside it.
root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
rel="$file"
case "$file" in
    "$root"/*) rel="${file#"$root"/}" ;;
esac

case "$rel" in
    */generated/*|generated/*|generated)
        deny "OpenWarrant: $rel is a generated projection. Edit the atoms and run \`war compile\` (AGENTS.md rule 4)." ;;
esac

[[ -f openwarrant.toml ]] || exit 0
if [[ -x ./target/debug/war ]]; then
    war_cmd=./target/debug/war
elif command -v war >/dev/null 2>&1; then
    war_cmd=war
else
    printf 'openwarrant guard-pins: no `war` binary (./target/debug/war or PATH); the pin guard did not run\n' >&2
    exit 0
fi

pinned=$("$war_cmd" --json pins --resolved-only 2>/dev/null | python3 -c 'import sys, json
rel = sys.argv[1]
try:
    pins = json.load(sys.stdin).get("result", {}).get("pins", [])
except Exception:
    sys.exit(0)
current = []
for p in pins:
    if p.get("path") != rel:
        continue
    # Absent means an older binary that knows no ownership: treat as current.
    if p.get("historical", False) is True:
        continue
    current.append(p.get("warrant", "?") + "/" + p.get("deliverable_id", "?"))
if current:
    print(current[0])' "$rel")
if [[ -n "$pinned" ]]; then
    deny "OpenWarrant: $rel is pinned by resolved $pinned and no later authorized Warrant governs it. It changes under a Warrant that declares it and is authorized (OW-ADR-0021), or through the correction act: edit it, run \`war correct ${pinned%/*} ${pinned#*/}\`, and a human signs (\`war sign ${pinned} --ssh-sign\`)."
fi
exit 0
