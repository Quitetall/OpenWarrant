#!/usr/bin/env bash
# What an agent's context pays for war's own words, in bytes (tokens ≈ bytes/4).
# Usage: tools/measure-tokens.sh [war-binary]   (default target/debug/war)
# Runs in a throwaway repository under $TMPDIR, removed on exit; touches
# nothing else. Prints one Markdown table.
set -euo pipefail
WAR=$(realpath "${1:-target/debug/war}")
for v in OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID \
         CLAUDE_CODE_SESSION_ID SSH_AUTH_SOCK SSH_AGENT_PID OPENWARRANT_ACTOR; do unset "$v"; done
export OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1
T=$(mktemp -d "${TMPDIR:-/tmp}/war-measure.XXXXXX")
trap '/usr/bin/rm -rf -- "$T"' EXIT
cd "$T" && git init -q && git config user.email m@example.invalid && git config user.name m
timeout 60 "$WAR" init >/dev/null 2>&1

b() { wc -c | tr -d ' '; }
row() { printf '| %s | %s | %s |\n' "$1" "$2" "$3"; }

# One MCP session: initialize, then each line of $2 as a request; prints replies.
mcp() {
    python3 -I - "$WAR" "$@" <<'PY'
import json, subprocess, sys
war, args, calls = sys.argv[1], sys.argv[2].split(), json.loads(sys.argv[3])
p = subprocess.Popen([war, "mcp", *args], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                     stderr=subprocess.DEVNULL, text=True)
def ask(i, method, params=None):
    msg = {"jsonrpc": "2.0", "id": i, "method": method}
    if params is not None: msg["params"] = params
    p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush()
    return json.loads(p.stdout.readline())
ask(0, "initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                      "clientInfo": {"name": "measure", "version": "0"}})
p.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); p.stdin.flush()
for i, (method, params) in enumerate(calls, 1):
    r = ask(i, method, params)
    print(json.dumps(r.get("result", r), separators=(",", ":"), ensure_ascii=False))
p.stdin.close(); p.wait(timeout=30)
PY
}
# Whether this build has the opt-in (`war mcp --all`); an older one served all.
ALL=""
if timeout 30 "$WAR" mcp --all --describe >/dev/null 2>&1; then ALL="--all"; fi

echo "| What | Bytes | ≈Tokens |"
echo "| --- | ---: | ---: |"
LIST=$(mcp "" '[["tools/list",{}]]')
n=$(printf '%s' "$LIST" | b); row "MCP tools/list, default ($(python3 -c 'import json,sys;print(len(json.loads(sys.argv[1])["tools"]))' "$LIST") tools)" "$n" $((n / 4))
if [[ -n $ALL ]]; then
    LIST=$(mcp "--all" '[["tools/list",{}]]')
    n=$(printf '%s' "$LIST" | b); row "MCP tools/list, --all ($(python3 -c 'import json,sys;print(len(json.loads(sys.argv[1])["tools"]))' "$LIST") tools)" "$n" $((n / 4))
fi
n=$(b < AGENTS.md); row "AGENTS.md from war init" "$n" $((n / 4))
n=$(timeout 30 "$WAR" admin agents-md --block --stdout 2>/dev/null | b); row "managed pointer block" "$n" $((n / 4))

# Text against envelope, the same question asked twice of the same state.
timeout 30 "$WAR" create "measure one" >/dev/null
timeout 30 "$WAR" create "measure two" >/dev/null
timeout 30 "$WAR" create "measure three" >/dev/null
for c in status check; do
    t=$(timeout 30 "$WAR" "$c" 2>/dev/null | b || true)
    j=$(timeout 30 "$WAR" --json "$c" 2>/dev/null | b || true)
    row "war $c: text / --json" "$t / $j" "$((t / 4)) / $((j / 4))"
done
# What answers "what is next" before it is asked: the overview, the ready rows
# alone, and the file an instruction-loading harness reads (n/a: not in this build).
t=$(timeout 30 "$WAR" view prime 2>/dev/null | b)
r=$(timeout 30 "$WAR" view prime --ready 2>/dev/null | b || true)
[[ $r -gt 0 ]] || r="n/a"
row "war view prime: overview / --ready" "$t / $r" "$((t / 4)) / $([[ $r == n/a ]] && echo n/a || echo $((r / 4)))"
t=$(OPENWARRANT_ACTOR=agent-a timeout 30 "$WAR" start 2>/dev/null | b)
if [[ -f .openwarrant/next.md ]]; then
    n=$(b < .openwarrant/next.md); row ".openwarrant/next.md, a held brief" "$n" $((n / 4))
else
    row ".openwarrant/next.md, a held brief" "n/a" "n/a"
fi
j=$(OPENWARRANT_ACTOR=agent-b timeout 30 "$WAR" --json start 2>/dev/null | b)
row "war start: text / --json" "$t / $j" "$((t / 4)) / $((j / 4))"
t=$(OPENWARRANT_ACTOR=agent-a timeout 30 "$WAR" done --note m 2>/dev/null | b)
j=$(OPENWARRANT_ACTOR=agent-b timeout 30 "$WAR" --json done --note m 2>/dev/null | b)
row "war done, naming the next: text / --json" "$t / $j" "$((t / 4)) / $((j / 4))"

# What one MCP reply costs: the default, and the envelope when asked.
for tool in war_status war_check; do
    d=$(mcp "" "[[\"tools/call\",{\"name\":\"$tool\",\"arguments\":{}}]]" | b)
    if [[ -n $ALL ]]; then
        e=$(mcp "" "[[\"tools/call\",{\"name\":\"$tool\",\"arguments\":{\"json\":true}}]]" | b)
    else
        e=$d
    fi
    row "MCP $tool reply: default / json" "$d / $e" "$((d / 4)) / $((e / 4))"
done
