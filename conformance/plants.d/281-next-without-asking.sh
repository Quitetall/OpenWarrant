# shellcheck shell=bash
# alpha.8 R3: what is next, without asking. Each claim beside its refusal:
#   `war done` that finishes a ticket names the next ready one, with the
#   command that takes it, and names nothing when nothing is ready (CLI and
#   MCP); a done that leaves the ticket open names no other ticket;
#   `war status` names ready tickets, never more than two, and none when
#   nothing is ready;
#   `war view prime --ready` is the ready rows alone, where plain prime
#   carries the how-to;
#   `.openwarrant/next.md` follows create, start and done (the held brief,
#   else the ready rows), is gone when nothing is held or ready, stays inside
#   6,144 bytes, and is never in `git status` nor staged by `git add -A`.

echo "== what is next, without asking (alpha.8 R3) =="
for v in OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID \
         CLAUDE_CODE_SESSION_ID OPENWARRANT_ACTOR; do unset "$v"; done
NX_T=$(mktemp -d)
nx_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
nx_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
NX_WAR="$REPO_ROOT/${WAR#./}"
NX_R="$NX_T/r"; mkdir -p "$NX_R"; git -C "$NX_R" init -q .
nxw() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE OPENWARRANT_NO_PROJECTS=1 OPENWARRANT_ACTOR=plant-agent \
    "$NX_WAR" --root "$NX_R" "$@" </dev/null; }
nx_id() { python3 -c 'import json,sys; v=json.load(sys.stdin)["result"]; print(v["id"], *[i["id"] for i in v.get("items", [])])'; }
nx_dirt() { git -C "$NX_R" status --porcelain --untracked-files=all; }
NX_F="$NX_R/.openwarrant/next.md"
nxw init --namespace NX >/dev/null 2>&1

# ---- nothing tracked: nothing said, no file ----------------------------------------
NX_ST=$(nxw status 2>/dev/null)
if [[ ! -e $NX_F ]] && ! grep -q 'war start' <<<"$NX_ST"; then
    nx_ok "nothing tracked, nothing offered" "no next.md, and status names no ticket"
else
    nx_fail "nothing tracked, nothing offered" "$(ls -A "$NX_R/.openwarrant" 2>/dev/null | tr '\n' ' ')"
fi

read -r NX_A NX_A1 NX_A2 <<<"$(nxw --json create "Alpha work" -i one -i two | nx_id)"
read -r NX_B _ <<<"$(nxw --json create "Beta work" | nx_id)"
read -r NX_C _ <<<"$(nxw --json create "Gamma work" | nx_id)"

# ---- status and prime --ready ------------------------------------------------------
NX_ST=$(nxw status 2>/dev/null)
if [[ $(grep -c '^  t-.*`war start t-' <<<"$NX_ST") == 2 ]] && grep -q '^Ready: 3 tickets' <<<"$NX_ST" \
    && [[ ${#NX_ST} -le 640 ]]; then
    nx_ok "status names two ready tickets" "three are ready, two are named, in ${#NX_ST} bytes"
else
    nx_fail "status names two ready tickets" "$(tr '\n' '|' <<<"$NX_ST")"
fi
NX_RD=$(nxw view prime --ready 2>/dev/null)
NX_PR=$(nxw view prime 2>/dev/null)
if [[ $(wc -l <<<"$NX_RD") == 3 && $(grep -c '`war start t-' <<<"$NX_RD") == 3 ]] && ! grep -q 'How to work' <<<"$NX_RD" \
    && grep -q 'How to work' <<<"$NX_PR"; then
    nx_ok "prime --ready is the rows alone" "three lines; plain prime still carries the how-to"
else
    nx_fail "prime --ready is the rows alone" "$(tr '\n' '|' <<<"$NX_RD")"
fi

# ---- next.md follows create and start, and is never a change ------------------------
NX_READY=$(cat "$NX_F" 2>/dev/null)
nxw start "$NX_A" >/dev/null 2>&1
NX_HELD=$(cat "$NX_F" 2>/dev/null)
git -C "$NX_R" add -A >/dev/null 2>&1
if [[ $NX_READY == "OpenWarrant: nothing is held here."* && $NX_READY == *"$NX_C"* ]] \
    && [[ $NX_HELD == "OpenWarrant: plant-agent holds $NX_A;"* && $NX_HELD == *"# $NX_A — Alpha work"* ]] \
    && [[ ${#NX_HELD} -le 6144 && $(nx_dirt) != *next.md* ]] \
    && ! git -C "$NX_R" ls-files --error-unmatch .openwarrant/next.md >/dev/null 2>&1; then
    nx_ok "next.md follows create and start" "the ready rows, then the held brief; not in git status, not staged by add -A"
else
    nx_fail "next.md follows create and start" "[$(head -1 <<<"$NX_READY")] [$(head -1 <<<"$NX_HELD")] $(nx_dirt | tr '\n' ' ')"
fi
git -C "$NX_R" -c user.email=p@invalid -c user.name=p commit -qm base >/dev/null 2>&1

# ---- done: an open ticket names no other; a finished one names the next -------------
NX_D1=$(nxw done "$NX_A/$NX_A1" 2>&1)
NX_D2=$(nxw done "$NX_A/$NX_A2" 2>&1)
NX_NEXT=$(sed -n 's/^next: \(t-[0-9a-f]*\) .*/\1/p' <<<"$NX_D2")
if ! grep -q '^next: ' <<<"$NX_D1" \
    && [[ $NX_NEXT == "$NX_B" || $NX_NEXT == "$NX_C" ]] \
    && grep -qE "^next: $NX_NEXT (Beta|Gamma) work \(\`war start $NX_NEXT\`\)$" <<<"$NX_D2" && [[ ${#NX_D2} -le 320 ]] \
    && [[ "$(cat "$NX_F")" == "OpenWarrant: nothing is held here."* ]]; then
    nx_ok "done names the next ready ticket" "$(grep '^next: ' <<<"$NX_D2"), in ${#NX_D2} bytes; next.md is the ready rows again"
else
    nx_fail "done names the next ready ticket" "$(tr '\n' '|' <<<"$NX_D1 // $NX_D2")"
fi

# ---- over MCP, then the last one: nothing is next -----------------------------------
NX_LAST=$NX_B; [[ $NX_NEXT == "$NX_B" ]] && NX_LAST=$NX_C
{
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"plant","version":"1"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"war_start","arguments":{"target":"%s","actor":"plant-agent"}}}\n' "$NX_NEXT"
    printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"war_done","arguments":{"target":"%s","actor":"plant-agent"}}}\n' "$NX_NEXT"
} > "$NX_T/tx"
NX_MCP=$(cd "$NX_R" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_NO_PROJECTS=1 timeout 60 python3 "$REPO_ROOT/conformance/fixtures/mcp/drive.py" "$NX_WAR" "$NX_T/tx" 2>/dev/null)
nxw start "$NX_LAST" >/dev/null 2>&1
NX_D3=$(nxw done "$NX_LAST" 2>&1); NX_S=$?
NX_ST=$(nxw status 2>/dev/null)
if grep -qF "next: $NX_LAST " <<<"$NX_MCP" && [[ $NX_S -eq 0 ]] && ! grep -q 'next:' <<<"$NX_D3" \
    && [[ ! -e $NX_F && $(nx_dirt) != *next.md* ]] && ! grep -q 'war start' <<<"$NX_ST"; then
    nx_ok "nothing ready, nothing named" "war_done named the last one; its done names none, status none, and next.md is gone"
else
    nx_fail "nothing ready, nothing named" "exit $NX_S: $(tr '\n' '|' <<<"$NX_D3") $(cut -c1-200 <<<"$NX_MCP")"
fi

command rm -rf "$NX_T"
unset NX_T NX_WAR NX_R NX_F NX_ST NX_A NX_A1 NX_A2 NX_B NX_C NX_RD NX_PR NX_READY NX_HELD NX_D1 NX_D2 NX_D3 NX_NEXT \
    NX_LAST NX_MCP NX_S
unset -f nx_ok nx_fail nxw nx_id nx_dirt
