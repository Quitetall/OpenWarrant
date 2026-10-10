#!/usr/bin/env bash
# PreToolUse hook (Edit|Write|MultiEdit|NotebookEdit): in a repository that
# uses OpenWarrant (an `openwarrant.toml` at its root), turn back an edit to a
# file a RESOLVED Warrant pins and no later authorized Warrant governs, or to
# anything under a generated/ directory. Anywhere else it says nothing: the
# plugin is installed per user, and a `generated/` directory in a repository
# that never adopted OpenWarrant is that repository's business (M9).
#
# Inside a worktree `war start` made (OW-WAR-0148 M15), whose session marker
# (`.openwarrant/session.json`) carries the paths its Warrant declares, an
# edit outside those paths is turned back too. A Warrant that declares no
# paths restricts nothing, and outside such a worktree the rule is silent.
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

# First: is this an OpenWarrant repository at all (its root, or the working
# directory for a program kept below a monorepo's root)? Nothing below
# applies to one that is not.
[[ -f "$root/openwarrant.toml" || -f openwarrant.toml ]] || exit 0

# The trees `war compile` writes into: docs/, and every directory the
# config's [paths] names. A `generated/` directory elsewhere (a build's
# codegen, say) is the repository's own, and an edit there is ordinary work.
cfg="$root/openwarrant.toml"
[[ -f "$cfg" ]] || cfg=openwarrant.toml
ow_tree() {
    case "$1" in docs/*) return 0 ;; esac
    local dir
    while IFS= read -r dir; do
        [[ -n "$dir" ]] || continue
        case "$1" in "${dir%/}"/*) return 0 ;; esac
    done < <(sed -n -E 's/^[[:space:]]*(sas|roadmap|adrs|warrants|gates|receipts)[[:space:]]*=[[:space:]]*"([^"]*)".*/\2/p' "$cfg")
    return 1
}
case "$rel" in
    */generated/*|generated/*)
        if ow_tree "$rel"; then
            deny "OpenWarrant: $rel is generated from the atoms, so an edit here would be overwritten. Edit the atoms and run \`war compile\`."
        fi ;;
esac

# Inside a `war start` worktree (OW-WAR-0148 M15): when the Warrant declares
# the paths its work touches, an edit outside them is turned back. The
# session marker is what `war start` wrote in this worktree with
# `[go] allowed_acts = "projected"`; with no marker, or a marker with no
# paths, nothing here applies. `**` matches across directories, `*` and `?`
# within one.
session="$root/.openwarrant/session.json"
[[ -f "$session" ]] || session=.openwarrant/session.json
if [[ -f "$session" ]]; then
    outside=$(python3 - "$session" "$rel" <<'PYGLOB'
import json, re, sys
try:
    s = json.load(open(sys.argv[1], encoding="utf-8"))
except Exception:
    sys.exit(0)
paths = s.get("paths") or []
rel = sys.argv[2]
def rx(glob):
    out, i = "", 0
    while i < len(glob):
        if glob.startswith("**", i):
            out += ".*"
            i += 2
        elif glob[i] == "*":
            out += "[^/]*"
            i += 1
        elif glob[i] == "?":
            out += "[^/]"
            i += 1
        else:
            out += re.escape(glob[i])
            i += 1
    return re.compile(out + r"\Z")
if paths and not any(rx(p.lstrip("/")).match(rel) for p in paths):
    print("%s|%s" % (s.get("node", "?"), ", ".join(paths)))
PYGLOB
)
    if [[ -n "$outside" ]]; then
        deny "OpenWarrant: this worktree was started for ${outside%%|*} (\`war start\`), and its Warrant declares the paths its work touches: ${outside#*|}. $rel is outside them, so the edit is turned back here; edit it from another checkout, or add the path to the Warrant's atoms/35-allowed.md."
    fi
fi
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
