#!/usr/bin/env bash
# SessionStart: put the agent's tracked work in front of it, so it needs no
# call to find it. `war view prime --session` prints nothing when no tracked
# work is open, or when the repository sets `[tickets] session_start = false`;
# then this hook adds nothing. Outside an OpenWarrant repository, or without
# `war`, it does nothing.
#
# It also records the commit the session started at, once per session (a
# resumed or compacted session keeps its first), under the git directory. The
# stop hook checks what changed since then, so work the agent committed during
# the session is checked too, not only what is uncommitted.
set -uo pipefail

input=$(cat)
field() {
    printf '%s' "$input" | python3 -c 'import sys, json
try:
    print(json.load(sys.stdin).get(sys.argv[1], ""))
except Exception:
    print("")' "$1"
}
cwd=$(field cwd)
if [[ -n "$cwd" ]] && ! cd "$cwd" 2>/dev/null; then
    exit 0
fi
[[ -f openwarrant.toml ]] || exit 0
session=$(field session_id)
if dir=$(git rev-parse --git-path openwarrant-stop 2>/dev/null) \
    && head=$(git rev-parse --verify --quiet HEAD 2>/dev/null); then
    base="$dir/${session:-default}.base"
    if [[ ! -s "$base" ]]; then
        mkdir -p "$dir" 2>/dev/null && printf '%s\n' "$head" > "$base" 2>/dev/null
    fi
fi
if [[ -x ./target/debug/war ]]; then
    war_cmd=./target/debug/war
elif command -v war >/dev/null 2>&1; then
    war_cmd=war
else
    exit 0
fi
out=$("$war_cmd" view prime --session 2>/dev/null) || exit 0
[[ -n "${out//[[:space:]]/}" ]] && printf '%s\n' "$out"
exit 0
