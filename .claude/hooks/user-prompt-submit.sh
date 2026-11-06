#!/usr/bin/env bash
# UserPromptSubmit: put the brief of the item a prompt names in front of the
# agent before its first turn, so a named item costs no call to find. Claude
# Code adds this hook's stdout to the context; a prompt that names nothing
# adds nothing. `war view prime --prompt` decides what a prompt names and what
# it prints, within its budget, and says nothing under
# `[tickets] session_start = false`.
#
# This hook only spares the agent repeats. The same output is added once a
# session, so a follow-up that names the item again costs nothing; an item
# whose text moved is new output and is added. A line listing several matches
# is added on a session's first prompt only: later prompts are conversation.
#
# It never holds a prompt. Exit 2 would block one, and this script has no path
# that returns it: whatever fails here, the prompt goes through as written.
# Notices stay for the agent's own next `war` call (`OPENWARRANT_NO_NOTICES`):
# delivered here they would go nowhere.
set -uo pipefail
trap 'exit 0' ERR

input=$(cat) || exit 0
read_field() {
    printf '%s' "$input" | python3 -c 'import sys, json
try:
    print(json.load(sys.stdin).get(sys.argv[1], ""))
except Exception:
    print("")' "$1" 2>/dev/null
}
cwd=$(read_field cwd)
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
limit=()
command -v timeout >/dev/null 2>&1 && limit=(timeout 5)
out=$(read_field prompt | OPENWARRANT_NO_NOTICES=1 OPENWARRANT_NO_UPDATE_CHECK=1 \
    ${limit[@]+"${limit[@]}"} "$war_cmd" view prime --prompt - 2>/dev/null) || exit 0

# Whether this is the session's first prompt, and what it was already given.
session=$(read_field session_id)
first=1
seen=""
if dir=$(git rev-parse --git-path openwarrant-stop 2>/dev/null); then
    seen="$dir/${session:-default}.briefs"
    [[ -e "$seen" ]] && first=0
    mkdir -p "$dir" 2>/dev/null && : >> "$seen" 2>/dev/null
fi
[[ -n "${out//[[:space:]]/}" ]] || exit 0
case "$out" in
    'OpenWarrant: that prompt could name '*) [[ $first == 1 ]] || exit 0 ;;
esac
print=$(printf '%s' "$out" | cksum | cut -d' ' -f1)
if [[ -n "$seen" ]]; then
    grep -qxF "$print" "$seen" 2>/dev/null && exit 0
    printf '%s\n' "$print" >> "$seen" 2>/dev/null
fi
printf '%s\n' "$out"
exit 0
