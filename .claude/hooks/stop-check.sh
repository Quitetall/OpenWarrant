#!/usr/bin/env bash
# Stop: check what the work changed, and say nothing when it holds.
#
# `war check --changed` checks only the Warrants whose records or pinned
# files differ from HEAD (the corpus, when a shared file such as the SAS or
# openwarrant.toml moved); a tree with nothing OpenWarrant governs changed is
# not checked at all. A clean result prints nothing, so a turn that broke
# nothing costs the agent no context and no `war check` call of its own.
#
# An ERROR or UNKNOWN holds the stop once: exit 2 with the brief report on
# stderr, which is what Claude Code hands the agent. The same findings do not
# hold a later stop of the same session (their fingerprint is kept under the
# git directory), and a stop that is already the answer to a hold is let
# through, so a finding only a person can answer never loops a session. A
# `war` that cannot run is said and never holds.
set -uo pipefail

input=$(cat)
case "$input" in
    *'"stop_hook_active":true'* | *'"stop_hook_active": true'*) exit 0 ;;
esac
field() {
    printf '%s' "$input" | sed -n "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p" | head -n 1
}
cwd=$(field cwd)
if [[ -n "$cwd" ]] && ! cd "$cwd" 2>/dev/null; then
    printf 'openwarrant stop-check: cannot enter %s; nothing was checked\n' "$cwd" >&2
    exit 0
fi
[[ -f openwarrant.toml ]] || exit 0
if [[ -x ./target/debug/war ]]; then
    war_cmd=./target/debug/war
elif command -v war >/dev/null 2>&1; then
    war_cmd=war
else
    printf 'OpenWarrant: war unavailable; what changed was not checked.\n' >&2
    exit 0
fi

# What changed since the session began (session-start.sh records the commit),
# so work committed during the session is in scope too; else since HEAD.
session=$(field session_id)
dir=$(git rev-parse --git-path openwarrant-stop 2>/dev/null) || dir=""
since=()
if [[ -n "$dir" && -s "$dir/${session:-default}.base" ]]; then
    base=$(head -n 1 "$dir/${session:-default}.base")
    if git rev-parse --verify --quiet "$base^{commit}" >/dev/null 2>&1; then
        since=(--since "$base")
    fi
fi
out=$("$war_cmd" --quiet check --changed ${since[@]+"${since[@]}"} 2>&1)
status=$?
case $status in
    0) ;;
    2)
        print=$(printf '%s' "$out" | cksum | cut -d' ' -f1)
        mark="$dir/${session:-default}"
        if [[ -n "$dir" && -f "$mark" && "$(cat "$mark" 2>/dev/null)" == "$print" ]]; then
            : # said once this session; nothing has moved since
        else
            if [[ -n "$dir" ]]; then
                mkdir -p "$dir" 2>/dev/null && printf '%s\n' "$print" > "$mark" 2>/dev/null
            fi
            {
                printf 'OpenWarrant: what changed does not check clean. Fix what you caused; a finding whose remedy is `human` is a person'"'"'s act: say so and stop.\n'
                printf '%s\n' "$out" | head -c 6000
            } >&2
            exit 2
        fi
        ;;
    *)
        printf 'OpenWarrant: `war check --changed` exited %s; what changed was not checked: %s\n' \
            "$status" "$(printf '%s' "$out" | head -c 300)" >&2
        exit 0
        ;;
esac

# OW-WAR-0148 M14: the end-of-session summary. Under a repo preset, what
# waits on a person (`war next` lists it) is shown to the person when the
# agent ends its turn: a message, not a block. Nothing without a preset.
if grep -q '^\[preset\]' openwarrant.toml 2>/dev/null; then
    waiting=$("$war_cmd" --json next 2>/dev/null | python3 -c 'import sys, json
try:
    v = json.load(sys.stdin)
except Exception:
    sys.exit(0)
acts = [a for a in ((v.get("result") or {}).get("actions") or []) if a.get("actor") == "human"]
if acts:
    first = "; ".join("%s %s" % (a.get("action", "?"), a.get("warrant", "?")) for a in acts[:3])
    more = "" if len(acts) <= 3 else " and %d more" % (len(acts) - 3)
    print(json.dumps({"systemMessage": "OpenWarrant: %d act(s) wait on a person (war next lists them): %s%s" % (len(acts), first, more)}))')
    if [[ -n "$waiting" ]]; then
        printf '%s\n' "$waiting"
    fi
fi
exit 0
