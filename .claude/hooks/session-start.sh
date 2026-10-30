#!/usr/bin/env bash
# SessionStart: put the agent's tracked work in front of it, so it needs no
# call to find it. `war view prime --session` prints nothing when no tracked
# work is open, or when the repository sets `[tickets] session_start = false`;
# then this hook adds nothing. Outside an OpenWarrant repository, or without
# `war`, it does nothing.
set -uo pipefail

input=$(cat)
cwd=$(printf '%s' "$input" | python3 -c 'import sys, json
try:
    print(json.load(sys.stdin).get("cwd", ""))
except Exception:
    print("")')
if [[ -n "$cwd" ]] && ! cd "$cwd" 2>/dev/null; then
    exit 0
fi
[[ -f openwarrant.toml ]] || exit 0
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
