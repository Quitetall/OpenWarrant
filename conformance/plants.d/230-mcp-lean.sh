# shellcheck shell=bash
# The lean MCP server (alpha.8). Token cost is a product measure. Each claim
# beside its refusal:
#   the default table is exactly the daily set, and no human act; a tool
#   outside it, and a signing tool, are "tool not found";
#   `--all` serves the whole table, still without a refused name;
#   a reply is the CLI's text, the envelope only when asked; a `json` that is
#   not a boolean is refused by the protocol;
#   a refusal names its rule id and the exit in that text;
#   the default tools/list stays inside 6,400 bytes, a budget `--all` breaks.

echo "== war mcp: the daily set, in the CLI's text =="
for v in OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID \
         CLAUDE_CODE_SESSION_ID OPENWARRANT_ACTOR; do unset "$v"; done
ML_T=$(mktemp -d)
ml_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ml_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ML_WAR=$(realpath "$WAR")
git -C "$ML_T" init -q . && (cd "$ML_T" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
    OPENWARRANT_NO_PROJECTS=1 "$ML_WAR" init >/dev/null 2>&1)

# One session: <out> <war mcp args...> <<< transcript lines (requests only).
ml_session() {
    local out="$1"; shift
    python3 - "$ML_T/tx.jsonl" > "$ML_T/tx.jsonl.tmp" <<'PY'
import json, sys
print(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
    "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "plant", "version": "0"}}}))
print(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}))
for n, line in enumerate(open(sys.argv[1], encoding="utf-8"), 2):
    method, params = json.loads(line)
    print(json.dumps({"jsonrpc": "2.0", "id": n, "method": method, "params": params}))
PY
    (cd "$ML_T" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_NO_PROJECTS=1 \
        timeout 60 python3 "$OLDPWD/conformance/fixtures/mcp/drive.py" "$ML_WAR" "$ML_T/tx.jsonl.tmp" "$@") \
        > "$out" 2>/dev/null
}
# Read reply <id> of <file> through a Python expression over r (the reply).
ml_read() {
    python3 -c "
import json, sys
r = next((m for m in map(json.loads, open(sys.argv[1])) if m.get('id') == int(sys.argv[2])), {})
res = r.get('result', {})
text = (res.get('content') or [{}])[0].get('text', '')
print($3)" "$1" "$2"
}

cat > "$ML_T/tx.jsonl" <<'EOF'
["tools/list", {}]
["tools/call", {"name": "war_next", "arguments": {}}]
["tools/call", {"name": "war_sign", "arguments": {"alias": "X"}}]
["tools/call", {"name": "war_status", "arguments": {}}]
["tools/call", {"name": "war_status", "arguments": {"json": true}}]
["tools/call", {"name": "war_done", "arguments": {"actor": "plant-a"}}]
["tools/call", {"name": "war_status", "arguments": {"json": "yes"}}]
EOF
ml_session "$ML_T/default.out"
ml_session "$ML_T/all.out" --all
REFUSED=$(python3 -c "
import re, sys
src = open(sys.argv[1]).read().split('pub const REFUSED_TOOLS', 1)[1].split('];', 1)[0]
print(' '.join(re.findall(r'\"(war_[a-z_]+)\"', src)))" crates/openwarrant-cli/src/mcp/mod.rs)

# The default table: exactly the seven, no refused or signing name; a tool
# outside it and a signing tool are not served.
ML_NAMES=$(ml_read "$ML_T/default.out" 2 "' '.join(sorted(t['name'] for t in res.get('tools', [])))")
ML_WANT="war_check war_context war_create war_done war_note war_start war_status"
ML_NEXT=$(ml_read "$ML_T/default.out" 3 "r.get('error', {}).get('message', '')")
ML_SIGN=$(ml_read "$ML_T/default.out" 4 "r.get('error', {}).get('message', '')")
ML_BAD=""
for r in $REFUSED; do [[ " $ML_NAMES " == *" $r "* ]] && ML_BAD="$ML_BAD $r"; done
if [[ "$ML_NAMES" == "$ML_WANT" && -z "$ML_BAD" && -n "$REFUSED" && "$ML_NEXT" == "tool not found" \
      && "$ML_SIGN" == "tool not found" ]]; then
    ml_ok "default table is the daily set" "7 tools, none of $(wc -w <<<"$REFUSED") refused names; war_next and war_sign not found"
else
    ml_fail "default table is the daily set" "[$ML_NAMES] refused:[$ML_BAD] next:[$ML_NEXT] sign:[$ML_SIGN]"
fi

# --all: the whole table, still without a refused or signing name.
ML_ALL=$(ml_read "$ML_T/all.out" 2 "' '.join(sorted(t['name'] for t in res.get('tools', [])))")
ML_ALLN=$(wc -w <<<"$ML_ALL")
ML_BAD=""
for r in $REFUSED; do [[ " $ML_ALL " == *" $r "* ]] && ML_BAD="$ML_BAD $r"; done
for n in $ML_ALL; do
    [[ $n == *ingest* ]] && ML_BAD="$ML_BAD $n"
    [[ $n == *sign* && $n != war_sign_list && $n != war_sign_show ]] && ML_BAD="$ML_BAD $n"
done
ML_ALLNEXT=$(ml_read "$ML_T/all.out" 3 "'error' not in r and 'nothing tracked' in text")
ML_ALLSIGN=$(ml_read "$ML_T/all.out" 4 "r.get('error', {}).get('message', '')")
if (( ML_ALLN >= 42 )) && [[ -z "$ML_BAD" && " $ML_ALL " == *" war_claim "* \
      && " $ML_ALL " == *" war_authorize_request "* && "$ML_ALLNEXT" == True && "$ML_ALLSIGN" == "tool not found" ]]; then
    ml_ok "--all serves the whole table" "$ML_ALLN tools, war_next answers, war_sign still not found"
else
    ml_fail "--all serves the whole table" "$ML_ALLN tools, bad:[$ML_BAD] next:$ML_ALLNEXT sign:[$ML_ALLSIGN]"
fi

# A reply is text; the envelope only when asked; a json that is no boolean
# is refused by the protocol.
ML_TEXT=$(ml_read "$ML_T/default.out" 5 "'structuredContent' not in res and text.startswith('Warrants:') and not text.startswith('{')")
ML_ENV=$(ml_read "$ML_T/default.out" 6 "res.get('structuredContent', {}).get('schema') == 'oh.war/report/v1' and json.loads(text)['command'] == 'status'")
ML_NOTB=$(ml_read "$ML_T/default.out" 8 "'error' in r")
if [[ "$ML_TEXT" == True && "$ML_ENV" == True && "$ML_NOTB" == True ]]; then
    ml_ok "a reply is text, envelope on ask" "war_status: text; json:true the envelope; json:\"yes\" refused"
else
    ml_fail "a reply is text, envelope on ask" "text:$ML_TEXT envelope:$ML_ENV json-not-bool:$ML_NOTB"
fi

# A refusal names its rule and the exit; the same call with json carries the
# rule in the envelope, so the text lost nothing.
ML_REF=$(ml_read "$ML_T/default.out" 7 "res.get('isError') is True and text.startswith('refused (ticket.nothing-held): ') and text.endswith('exit 2, not_ready')")
ML_OK=$(ml_read "$ML_T/default.out" 5 "'refused (' not in text and 'exit ' not in text")
if [[ "$ML_REF" == True && "$ML_OK" == True ]]; then
    ml_ok "a refusal names its rule id" "refused (ticket.nothing-held): ... exit 2, not_ready; a clean reply carries neither"
else
    ml_fail "a refusal names its rule id" "refusal:$ML_REF clean:$ML_OK"
fi

# The budget: the default tools/list result inside 6,400 bytes; --all is far
# past it, so the measure tells the two apart.
ML_DB=$(ml_read "$ML_T/default.out" 2 "len(json.dumps(res, separators=(',', ':'), ensure_ascii=False).encode())")
ML_AB=$(ml_read "$ML_T/all.out" 2 "len(json.dumps(res, separators=(',', ':'), ensure_ascii=False).encode())")
if (( ML_DB > 0 && ML_DB <= 6400 && ML_AB > 6400 )); then
    ml_ok "default tools/list in budget" "$ML_DB bytes (<= 6400); --all $ML_AB"
else
    ml_fail "default tools/list in budget" "default $ML_DB, --all $ML_AB"
fi

/usr/bin/rm -rf -- "$ML_T"
unset ML_T ML_WAR ML_NAMES ML_WANT ML_NEXT ML_SIGN ML_BAD ML_ALL ML_ALLN ML_ALLNEXT ML_ALLSIGN \
    ML_TEXT ML_ENV ML_NOTB ML_REF ML_OK ML_DB ML_AB REFUSED
unset -f ml_ok ml_fail ml_session ml_read
