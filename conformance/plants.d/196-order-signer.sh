# shellcheck shell=bash
# t-3c16c W1 — an Order reaches its signer.
#
# `war done` on an Order does what an agent may, leaves the signature item open, and
# exits 0 with one line naming the command. After that `sign --list`, `status`, `view
# next` (actor human) and the board name it; the signature closes it with no agent
# step. Refused: an agent ticking or signing the item; a failing test, with items or
# with none, reaching the queue; an Order nobody finished reaching it; an item added
# after the handover keeping it there. Throwaway key; session variables cleared.

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

/usr/bin/rm -rf "$OS_T"
