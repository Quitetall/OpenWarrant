#!/usr/bin/env bash
# TaskCompleted hook (OW-WAR-0148 M13): when Claude Code marks one of its own
# tasks completed and the task names a Warrant's item (`t-x/i-y`, or an `i-y`
# one ticket has), tick that item through `war done --check`: at observed
# when its tests and KPIs pass, as claimed when it has nothing to check. The
# bridge (`war bridge claude-tasks --event - --apply`) decides; this script
# only hands it the event.
#
# It never holds the task: it exits 0 whatever happens, and writes nothing on
# stdout (exit code 2 would hold the completion; this script has no path that
# returns it). A task that names nothing of this repository, a repository
# without openwarrant.toml, or no `war` on hand: it does nothing at all. A
# tick the bridge could not make (a failing test, a claim someone else
# holds) is said on stderr, and the task completes regardless.
#
# The input fields are Claude Code's: task_id, task_subject and, when there
# is one, task_description (code.claude.com/docs/en/hooks, "TaskCompleted").
set -uo pipefail

input=$(cat)
cwd=$(printf '%s' "$input" | python3 -c 'import sys, json
try:
    print(json.load(sys.stdin).get("cwd", ""))
except Exception:
    print("")' 2>/dev/null)
if [[ -n "$cwd" ]] && ! cd "$cwd" 2>/dev/null; then
    exit 0
fi
root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
[[ -f "$root/openwarrant.toml" || -f openwarrant.toml ]] || exit 0
# Cheap first: a task that names no item id is not ours.
id_re='(^|[^a-z0-9-])(t-[0-9a-z]{3,16}/)?i-[0-9a-z]{3,16}([^a-z0-9-]|$)|(^|[^a-z0-9/-])t-[0-9a-z]{3,16}([^a-z0-9/-]|$)'
[[ "$input" =~ $id_re ]] || exit 0
if [[ -x "$root/target/debug/war" ]]; then
    war_cmd="$root/target/debug/war"
elif command -v war >/dev/null 2>&1; then
    war_cmd=war
else
    printf 'openwarrant task-completed: no `war` binary; nothing was ticked\n' >&2
    exit 0
fi
said=$(printf '%s' "$input" | "$war_cmd" bridge claude-tasks --event - --apply 2>&1)
if [[ -n "$said" ]]; then
    printf 'openwarrant: %s\n' "$(printf '%s' "$said" | head -c 600)" >&2
fi
exit 0
