# shellcheck shell=bash
# t-67ed — the web UI's Tickets page, on the loopback page.
#
# Its own file, beside 63-webui.sh rather than inside it: OW-WAR-0139's
# OBL-005 pins 63-webui.sh's bytes. The server runs over a scratch program
# with SSH_AUTH_SOCK unset; a ticket act is not a signing act and never
# reaches a key. The LAN refusal is in 59-webui-lan.sh.

echo "== web ui tickets (t-67ed) =="
PLANT_ROOT=$(scratch_corpus WT)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
"$WAR" --root "$PLANT_ROOT" create "Web ticket" --item "Web item" >/dev/null 2>&1
WT_TT=$("$WAR" --root "$PLANT_ROOT" --json ready 2>/dev/null | python3 -c '
import sys, json
r = json.load(sys.stdin)["result"]["ready"]
print(next(x["ticket"] + "/" + x["item"] for x in r if x["text"] == "Web item"))' 2>/dev/null)
WT_CL="$PLANT_ROOT/docs/tickets/${WT_TT%%/*}/atoms/15-checklist.md"

WT_OUT=$(mktemp); WT_ERR=$(mktemp)
env -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" ui --port 0 --as web-plant >"$WT_OUT" 2>"$WT_ERR" &
WT_PID=$!
for _ in $(seq 1 60); do grep -q 'http://' "$WT_OUT" && break; sleep 0.25; done
WT_URL=$(grep -o 'http://[^ ]*' "$WT_OUT" | head -1)
WT_HOST=${WT_URL#http://}; WT_HOST=${WT_HOST%%/*}
WT_TOK=${WT_URL#*#t=}; WT_TOK=${WT_TOK%%&*}
WT_B="http://$WT_HOST"
wt_expect() { # name, got, want
    if [[ "$2" == "$3" ]]; then
        printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s got %s, wanted %s\n' "$1" "$2" "$3"; FAILED=$((FAILED + 1))
    fi
}
wt_post() { curl -s -o /dev/null -w '%{http_code}' -X POST -H "Authorization: Bearer $WT_TOK" -H 'Content-Type: application/json' "$@" "$WT_B/api/ticket"; }

if [[ -z "$WT_URL" || -z "$WT_TT" ]]; then
    printf 'FAIL  %-34s url %s, ticket %s: %s\n' "war ui starts, a ticket exists" "$WT_URL" "$WT_TT" "$(tail -2 "$WT_ERR")"
    FAILED=$((FAILED + 1))
else
    wt_expect "the page opens on Tickets" "$(grep -c 'p=tickets' "$WT_OUT")" 1
    WT_TV=$(curl -s -H "Authorization: Bearer $WT_TOK" "$WT_B/api/tickets")
    if python3 -c '
import sys, json
d = json.load(sys.stdin)
t = [x for x in d["tickets"] if x["ticket"]["title"] == "Web ticket"][0]
assert t["items"][0]["text"] == "Web item" and t["items"][0]["ready"] is True
' <<<"$WT_TV" 2>/dev/null; then
        printf 'ok    %-34s the ticket, its item, ready\n' "the tickets view"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "the tickets view" "$(head -c 200 <<<"$WT_TV")"; FAILED=$((FAILED + 1))
    fi
    # Refusals first: nothing below may write.
    wt_expect "a ticket act with no token" "$(curl -s -o /dev/null -w '%{http_code}' -X POST -H "Origin: $WT_B" -d "{\"act\":\"claim\",\"target\":\"$WT_TT\"}" "$WT_B/api/ticket")" 401
    wt_expect "a ticket act with no Origin" "$(wt_post -d "{\"act\":\"claim\",\"target\":\"$WT_TT\"}")" 403
    wt_expect "a ticket act, foreign Origin" "$(wt_post -H 'Origin: http://evil.example' -d "{\"act\":\"claim\",\"target\":\"$WT_TT\"}")" 403
    wt_expect "a ticket act naming another act" "$(wt_post -H "Origin: $WT_B" -d "{\"act\":\"sign\",\"target\":\"$WT_TT\"}")" 400
    wt_expect "a ticket act carrying an argv" "$(wt_post -H "Origin: $WT_B" -d "{\"act\":\"claim\",\"target\":\"$WT_TT\",\"argv\":[\"rm\"]}")" 400
    wt_expect "a ticket act on a non-ticket" "$(wt_post -H "Origin: $WT_B" -d '{"act":"claim","target":"WT-WAR-0001"}')" 400
    wt_expect "a ticket act by GET" "$(curl -s -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $WT_TOK" "$WT_B/api/ticket")" 405
    wt_expect "done before any claim" "$(wt_post -H "Origin: $WT_B" -d "{\"act\":\"done\",\"target\":\"$WT_TT\"}")" 409
    if grep -q '^- \[ \] Web item' "$WT_CL" && [[ -z "$(ls -A "$PLANT_ROOT/.openwarrant/state/claims" 2>/dev/null)" ]]; then
        printf 'ok    %-34s no claim, box unticked\n' "the refusals wrote nothing"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "the refusals wrote nothing" "$(grep 'Web item' "$WT_CL")"; FAILED=$((FAILED + 1))
    fi
    # Accepting: claim, then done with a note, as the CLI would.
    wt_expect "the loopback page claims" "$(wt_post -H "Origin: $WT_B" -d "{\"act\":\"claim\",\"target\":\"$WT_TT\"}")" 200
    wt_expect "the loopback page finishes" "$(wt_post -H "Origin: $WT_B" -d "{\"act\":\"done\",\"target\":\"$WT_TT\",\"note\":\"from the page\"}")" 200
    if grep -q '^- \[x\] Web item .*done by web-plant.*from the page' "$WT_CL"; then
        printf 'ok    %-34s ticked by web-plant, with the note\n' "the checklist records it"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "the checklist records it" "$(grep 'Web item' "$WT_CL")"; FAILED=$((FAILED + 1))
    fi
    # A repeat by the same actor is `war done`'s idempotent answer, not a
    # second write: the box is ticked once.
    WT_AGAIN=$(curl -s -X POST -H "Authorization: Bearer $WT_TOK" -H "Origin: $WT_B" -H 'Content-Type: application/json' \
        -d "{\"act\":\"done\",\"target\":\"$WT_TT\"}" "$WT_B/api/ticket")
    if grep -q 'already done' <<<"$WT_AGAIN" && [[ $(grep -c 'Web item' "$WT_CL") == 1 ]]; then
        printf 'ok    %-34s already done; one ticked line\n' "a repeated done writes nothing"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "a repeated done writes nothing" "$WT_AGAIN"; FAILED=$((FAILED + 1))
    fi
fi
kill "$WT_PID" 2>/dev/null; wait "$WT_PID" 2>/dev/null
command rm -f "$WT_OUT" "$WT_ERR"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
