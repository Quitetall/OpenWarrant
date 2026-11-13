# shellcheck shell=bash
# t-3c16c W1 — an Order reaches its signer.
#
# `war done` on an Order does what an agent may, leaves the signature item open, and
# exits 0 with one line naming the command. After that `sign --list`, `status`, `view
# next` (actor human) and the board name it; the signature closes it with no agent
# step. Refused: an agent ticking or signing the item; a failing test, with items or
# with none, reaching the queue; an Order nobody finished reaching it; an item added
# after the handover keeping it there. Throwaway key; session variables cleared.
#
# Also: `war status --json` and `war plan frontier` list single-file Warrants and
# Orders, and refuse a plain ticket that place; nothing offers to dispatch one;
# `war_create` takes `sign`.

# This plant is about tickets kept as `docs/tickets/<t-id>.md`, the layout before
# `docs/work/`, which stays readable and workable: `war create` writes it here.
# 293-create-master.sh runs the loop on a master.
export OPENWARRANT_TICKET_LAYOUT=tickets

echo "== an Order reaches its signer (t-3c16c W1) =="
OS_ROOT=$(scratch_corpus OS)
[[ -d "${OS_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
OS_WAR="$REPO_ROOT/${WAR#./}"
OS_T=$(mktemp -d)
os_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
os_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
os_check() { if [[ $2 == 0 ]]; then os_ok "$1" "$3"; else os_fail "$1" "$3: $(head -3 "$OS_T/out" "$OS_T/err" | tr '\n' '|' | cut -c1-300)"; fi; }
# os_as <actor> <war args...>: stdout to $OS_T/out, stderr to $OS_T/err.
os_as() {
    local actor="$1"; shift
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDE_CODE_SESSION_ID -u CODEX_THREAD_ID \
        -u CODEX_SESSION_ID -u KATANA_SESSION_ID -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR="$actor" \
        OPENWARRANT_SESSION_ID=plant-196 "$OS_WAR" --root "$OS_ROOT" "$@" >"$OS_T/out" 2>"$OS_T/err" </dev/null
}
os_py() { python3 -c 'import json,sys
v=json.load(open(sys.argv[1]))
print(eval(sys.argv[2]))' "$OS_T/out" "$1"; }
os_create() { os_as owner --json create "$@" && os_py 'v["result"]["id"]'; }
os_sign_item() { os_as owner --json view show "$1"; os_py 'next(i["id"] for i in v["result"]["items"] if i["text"].startswith("Signed off"))'; }
os_open() { os_as owner --json view show "$1"; os_py 'sum(not i["done"] for i in v["result"]["items"])'; }
os_says() { grep -qF -- "$1" "$OS_T/out"; }
os_listed() { os_as owner sign --list; os_says "$1"; }

ssh-keygen -q -t ed25519 -N "" -C plant-ada -f "$OS_T/id_ada" </dev/null \
    || { printf 'PLANT SETUP FAILED: ssh-keygen\n' >&2; exit 9; }
OS_PUB=$(cut -d' ' -f1,2 "$OS_T/id_ada.pub")
printf 'ada namespaces="oh.war/response,oh.war/dsse" %s\n' "$OS_PUB" >"$OS_ROOT/docs/authority/allowed_signers"
cat >"$OS_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Ada"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/196-order-signer.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "ada"
ROLES
printf '\n[roles.roster]\nada = "maintain"\n' >>"$OS_ROOT/openwarrant.toml"

# A person's act, in an agent holding one throwaway key and nothing else.
os_human() {
    (
        eval "$(ssh-agent -s)" >/dev/null
        trap 'ssh-agent -k >/dev/null 2>&1' EXIT
        ssh-add -q "$OS_T/id_ada" 2>/dev/null
        [[ "$(ssh-add -L 2>/dev/null)" == "$OS_PUB"* && $(ssh-add -L | wc -l) == 1 ]] \
            || { echo 'PLANT: the agent holds the wrong keys' >&2; exit 97; }
        OPENWARRANT_ACTOR=Ada "$OS_WAR" --root "$OS_ROOT" "$@" </dev/null
    ) >"$OS_T/human.out" 2>"$OS_T/human.err"
}

os_as owner status
OS_BASE=$(sed -n 's/^Awaiting a signature: //p' "$OS_T/out")

# ---- nobody finished it: not in the queue ------------------------------------------------
OS_A=$(os_create "An Order with work" --sign --item work)
OS_AS=$(os_sign_item "$OS_A")
if ! os_listed "$OS_A"; then
    os_ok "an unfinished Order is not queued" "created, not done: sign --list does not name it"
else
    os_fail "an unfinished Order is not queued" "$(head -3 "$OS_T/out" | tr '\n' '|')"
fi

# ---- done hands it over -------------------------------------------------------------------
os_as agent start "$OS_A"
os_as agent done "$OS_A" --note fin; OS_RC=$?
OS_LINE="$OS_A waits on a person: \`war sign $OS_A/$OS_AS --ssh-sign\`"
if [[ $OS_RC == 0 ]] && os_says "$OS_LINE" && [[ $(os_open "$OS_A") == 1 ]]; then
    os_ok "done hands an Order to its signer" "exit 0; $OS_LINE"
else
    os_fail "done hands an Order to its signer" "exit $OS_RC: $(head -4 "$OS_T/out" "$OS_T/err" | tr '\n' '|')"
fi
os_as agent --json done "$OS_A"; OS_RC=$?
OS_STATE=$(os_py 'v["result"]["ticket_state"]+" "+v["result"]["awaiting_signature"][0]')
OS_EVENTS=$(cat "$OS_ROOT"/docs/tickets/*/journal.jsonl "$OS_ROOT"/docs/tickets/.journals/* 2>/dev/null | grep -c 'work.awaits_signature')
[[ $OS_RC == 0 && $OS_STATE == "awaiting_signature $OS_A/$OS_AS" && $OS_EVENTS == 1 ]]
os_check "a repeat says the same, writes nothing" $? "exit $OS_RC, $OS_STATE, $OS_EVENTS event(s)"

# ---- every reader names it ----------------------------------------------------------------
os_listed "$OS_A/$OS_AS"; os_check "sign --list names it" $? "$(grep -F "$OS_A" "$OS_T/out")"
os_as owner status; grep -qx "Awaiting a signature: $((OS_BASE + 1))" "$OS_T/out"
os_check "status counts it" $? "$OS_BASE before, then $(grep -F Awaiting "$OS_T/out")"
os_as owner --json view next
[[ $(os_py '[a["actor"] for a in v["result"]["actions"] if a["warrant"]==sys.argv[0] or "'"$OS_A"'" in a["command"]]') == "['human']" ]] \
    && [[ $(os_py '[r for r in v["result"].get("ready",[]) if r["ticket"]=="'"$OS_A"'"]') == "[]" ]]
os_check "view next: a person's, not ready" $? "actor human; not offered to an agent"
os_as owner --json view console
[[ $(os_py '[a["command"] for a in v["result"]["acts"] if a["act"]=="sign_off"]') == "['war sign $OS_A/$OS_AS --ssh-sign']" ]]
os_check "the signing queue the TUI reads" $? "console board: the exact command"

# ---- what an agent cannot do --------------------------------------------------------------
os_as agent admin claim "$OS_A/$OS_AS"
os_as agent done "$OS_A/$OS_AS"; OS_RC=$?
OS_ERR=$(cat "$OS_T/err")
os_as agent admin release "$OS_A/$OS_AS"
if [[ $OS_RC == 2 && $OS_ERR == *ticket.tick-below-minimum* ]] && [[ $(os_open "$OS_A") == 1 ]]; then
    os_ok "an agent cannot tick the signature" "exit 2, ticket.tick-below-minimum; still open"
else
    os_fail "an agent cannot tick the signature" "exit $OS_RC: $(head -2 "$OS_T/err" | tr '\n' '|')"
fi
os_as agent sign "$OS_A/$OS_AS" --ssh-sign --as Ada; OS_RC=$?
if [[ $OS_RC != 0 ]] && [[ $(os_open "$OS_A") == 1 ]]; then
    os_ok "an agent cannot sign it" "with no key in reach: exit $OS_RC"
else
    os_fail "an agent cannot sign it" "exit $OS_RC"
fi

# ---- the signature closes it --------------------------------------------------------------
os_human sign "$OS_A/$OS_AS" --ssh-sign --as Ada; OS_RC=$?
os_as owner --json view show "$OS_A"
OS_DONE=$(os_py 'v["result"]["ticket"]["state"]')
if [[ $OS_RC == 0 && $OS_DONE == done ]] && ! os_listed "$OS_A"; then
    os_ok "the signature closes it" "no further agent step: done, and out of the queue"
else
    os_fail "the signature closes it" "exit $OS_RC, state $OS_DONE: $(head -3 "$OS_T/human.err" | tr '\n' '|')"
fi

# ---- a failing test stays out of the queue ---------------------------------------------------
OS_B=$(os_create "An Order whose test fails" --sign --item work --test 'test -e passes')
OS_C=$(os_create "An Order with no item of its own" --sign --test 'test -e passes')
os_as agent start "$OS_B"; os_as agent done "$OS_B"; OS_RC=$?
os_as agent start "$OS_C"; os_as agent done "$OS_C"; OS_RC2=$?
OS_ERR=$(cat "$OS_T/err")
if [[ $OS_RC == 2 && $OS_RC2 == 2 && $(os_open "$OS_B") == 2 && $OS_ERR == *check.failed* ]] \
    && ! os_listed "$OS_B" && ! os_says "$OS_C"; then
    os_ok "a failing test is not queued" "both refused, nothing ticked, sign --list names neither"
else
    os_fail "a failing test is not queued" "$OS_RC/$OS_RC2: $(head -3 "$OS_T/err" "$OS_T/out" | tr '\n' '|')"
fi
: >"$OS_ROOT/passes"
os_as agent done "$OS_B"; OS_RC=$?
os_as agent done "$OS_C"; OS_RC2=$?
[[ $OS_RC == 0 && $OS_RC2 == 0 ]] && os_listed "$OS_B" && os_says "$OS_C"
os_check "the test passing queues it" $? "exit $OS_RC/$OS_RC2; both listed"

# ---- the last item named; work added after -----------------------------------------------
OS_D=$(os_create "An Order worked item by item" --sign --item one --item two)
os_as owner --json view show "$OS_D"
OS_D1=$(os_py 'next(i["id"] for i in v["result"]["items"] if i["text"]=="one")')
OS_D2=$(os_py 'next(i["id"] for i in v["result"]["items"] if i["text"]=="two")')
os_as agent start "$OS_D"
os_as agent done "$OS_D/$OS_D1"; OS_RC=$?
if [[ $OS_RC == 0 ]] && ! os_listed "$OS_D"; then
    os_ok "an item left keeps it from the queue" "$OS_D/$OS_D1 ticked, $OS_D/$OS_D2 open: not queued"
else
    os_fail "an item left keeps it from the queue" "exit $OS_RC: $(head -3 "$OS_T/out" | tr '\n' '|')"
fi
os_as agent done "$OS_D/$OS_D2"; OS_RC=$?
os_says "$OS_D waits on a person"; OS_SAID=$?
[[ $OS_RC == 0 && $OS_SAID == 0 ]] && os_listed "$OS_D"
os_check "the last item named hands it over" $? "war done $OS_D/$OS_D2 named the command"
os_as owner add "$OS_D" "more work"
if ! os_listed "$OS_D"; then
    os_ok "work added takes it back" "an open item of the agent's: out of the queue"
else
    os_fail "work added takes it back" "$(head -3 "$OS_T/out" | tr '\n' '|')"
fi

# ---- single-file Warrants and Orders, in status and the frontier ------------------------
OS_E=$(os_create "A Warrant in one document" --test true)
OS_F=$(os_create "A plain ticket")
os_as owner --json plan new "A kept document" --format 1.0
OS_G=$(os_py 'v["result"].get("warrant") or v["result"].get("alias") or v["result"].get("id")')
os_as owner --json status
OS_LISTED=$(os_py '" ".join(i["id"] for i in v["result"].get("single_file",[]))')
OS_STATE=$(os_py 'next(i["state"] for i in v["result"]["single_file"] if i["id"]=="'"$OS_A"'")')
if [[ " $OS_LISTED " == *" $OS_E "* && " $OS_LISTED " == *" $OS_A "* && " $OS_LISTED " == *" $OS_G "* \
    && " $OS_LISTED " != *" $OS_F "* && $OS_STATE == done ]]; then
    os_ok "status lists single-file work" "a Warrant, an Order and a document; a plain ticket is not one"
else
    os_fail "status lists single-file work" "[$OS_LISTED] e=$OS_E g=$OS_G f=$OS_F a=$OS_A/$OS_STATE"
fi
os_as owner --json plan frontier
OS_LISTED=$(os_py '" ".join(i["id"] for i in v["result"].get("documents",[]))')
os_as owner plan frontier; OS_HUMAN=$(cat "$OS_T/out")
if [[ " $OS_LISTED " == *" $OS_E "* && " $OS_LISTED " == *" $OS_G "* && " $OS_LISTED " != *" $OS_F "* \
    && " $OS_LISTED " != *" $OS_A "* && $OS_HUMAN == *"$OS_E"* ]]; then
    os_ok "the frontier lists what is open" "open ones listed; a closed Order and a plain ticket are not"
else
    os_fail "the frontier lists what is open" "[$OS_LISTED] e=$OS_E g=$OS_G"
fi
os_as owner --json view console
if [[ $(os_py '[s for s in v["result"]["stages"] if s["warrant"]=="'"$OS_E"'"]') == "[]" ]]; then
    os_ok "nothing offers to dispatch them" "no stage row, so no command that could not run"
else
    os_fail "nothing offers to dispatch them" "$(head -3 "$OS_T/out" | tr '\n' '|')"
fi

# ---- --check on a ticket: its test applies to every item, one added since included ------------
OS_H=$(os_create "A ticket tested as a whole" --item first)
os_as owner plan add "$OS_H" --test 'test -e passes'
os_as owner plan add "$OS_H" "added after the test"
/usr/bin/rm -f "$OS_ROOT/passes"
os_as agent start "$OS_H"
os_as agent done "$OS_H" --check; OS_RC=$?
OS_ERR=$(cat "$OS_T/err")
if [[ $OS_RC == 2 && $OS_ERR == *fail* && $OS_ERR != *ticket.check-nothing* && $(os_open "$OS_H") == 2 ]]; then
    os_ok "--check runs a ticket's test" "it fails: exit 2, nothing ticked, not check-nothing"
else
    os_fail "--check runs a ticket's test" "exit $OS_RC: $OS_ERR"
fi
: >"$OS_ROOT/passes"
os_as agent done "$OS_H" --check; OS_RC=$?
os_as owner --json view show "$OS_H"
OS_ALL=$(os_py '[i["done"] for i in v["result"]["items"]]')
OS_SEEN=$(grep -c '\[observed\]' "$(tk "$OS_ROOT/docs/tickets/$OS_H.md")")
if [[ $OS_RC == 0 && $OS_ALL == "[True, True]" && $OS_SEEN == 2 ]]; then
    os_ok "--check ticks every item observed" "$OS_H/both, the one added since included"
else
    os_fail "--check ticks every item observed" "exit $OS_RC, $OS_ALL: $(cut -c1-200 "$OS_T/err")"
fi
OS_I=$(os_create "A ticket with no test" --item only)
os_as agent start "$OS_I"
os_as agent done "$OS_I" --check; OS_RC=$?
if [[ $OS_RC == 2 ]] && grep -qF 'ticket.check-nothing' "$OS_T/err"; then
    os_ok "--check with no test is refused" "ticket.check-nothing still stands"
else
    os_fail "--check with no test is refused" "exit $OS_RC: $(cut -c1-200 "$OS_T/err")"
fi

# ---- war create names the document ---------------------------------------------------------
os_as owner create "A named document"
OS_MADE=$(sed -n 's/^created //p' "$OS_T/out")
if [[ $OS_MADE == docs/tickets/t-*.md && -f "$OS_ROOT/$OS_MADE" ]]; then
    os_ok "create prints the document's path" "$OS_MADE exists"
else
    os_fail "create prints the document's path" "[$OS_MADE]"
fi

# ---- war_create over MCP asks for the same ---------------------------------------------------
OS_TX="$OS_T/tx"
{
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"plant","version":"1"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"war_create","arguments":{"title":"An Order over MCP","sign":true,"actor":"mcp-agent"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"war_create","arguments":{"title":"A ticket over MCP","sign":false,"actor":"mcp-agent"}}}'
} >"$OS_TX"
OS_MCP=$(cd "$OS_ROOT" && env -u OPENWARRANT_ACTOR python3 "$REPO_ROOT/conformance/fixtures/mcp/drive.py" "$OS_WAR" "$OS_TX" 2>/dev/null)
OS_ORDER=$(grep -l '^# An Order over MCP$' "$OS_ROOT"/docs/tickets/*.md 2>/dev/null | head -1)
OS_TICKET=$(grep -l '^# A ticket over MCP$' "$OS_ROOT"/docs/tickets/*.md 2>/dev/null | head -1)
if [[ -n $OS_ORDER && -n $OS_TICKET ]] && grep -q '^min: signed$' "$OS_ORDER" && ! grep -q '^min: signed$' "$OS_TICKET"; then
    os_ok "war_create takes sign" "sign true writes min: signed; false writes none"
else
    os_fail "war_create takes sign" "$(head -c 300 <<<"$OS_MCP")"
fi

# ---- alpha.8 R1: the default test stands before the queue, never in place of the signer -----
printf '\n[tickets]\ndone_test = "test -f default-ok"\n' >>"$OS_ROOT/openwarrant.toml"
OS_G=$(os_create "An Order under the default test" --sign --item one)
os_as agent start "$OS_G"
os_as agent done "$OS_G"; OS_RC=$?
OS_ERR=$(cat "$OS_T/err")
os_listed "$OS_G"; OS_QUEUED=$?
: >"$OS_ROOT/default-ok"
os_as agent done "$OS_G"; OS_RC2=$?
OS_OUT=$(cat "$OS_T/out")
if [[ $OS_RC == 2 && $OS_ERR == *ticket.check-failed* && $OS_QUEUED != 0 && $OS_RC2 == 0 \
    && $OS_OUT == *"ticked at observed: default passed"* && $OS_OUT == *"$OS_G waits on a person"* ]] \
    && os_listed "$OS_G" && [[ $(os_open "$OS_G") == 1 ]]; then
    os_ok "the default test, then the signer" "failing: not queued; passing: the agent's item observed, the Order queued, its signature item open"
else
    os_fail "the default test, then the signer" "exit $OS_RC (queued $OS_QUEUED) then $OS_RC2: $(head -c 300 <<<"$OS_OUT$OS_ERR")"
fi

/usr/bin/rm -rf "$OS_T"
unset OPENWARRANT_TICKET_LAYOUT
