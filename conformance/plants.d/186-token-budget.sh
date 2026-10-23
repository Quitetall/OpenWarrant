# shellcheck shell=bash
# What an agent reads to take one Warrant, held to a budget (2026-10-23, the
# adoption pilot: OpenWarrant's loop cost 47% more tokens than a plain
# CLAUDE.md, most of it reading the same backlog three times over).
#
# Each claim is paired with an observed refusal:
# - `war view prime` is one line per open Warrant: at most 200 bytes a Warrant
#   plus a 600-byte header, however long their text; a marker planted deep in
#   one Warrant's description and items is not in it.
# - `war start` prints that one Warrant: the marker is in its brief, the
#   brief is at most its own text plus 700 bytes, and another Warrant's marker
#   is not in it.
# - `war view ready` is one line per ready Warrant, never an item's text.
# - `war done` with no id closes the Warrant held in one call; a second
#   `war done` holds nothing and is refused by name (ticket.nothing-held).

echo "== what an agent reads to take one Warrant (token budget) =="
TB_TMP=$(mktemp -d)
TB_WAR="$REPO_ROOT/${WAR#./}"
tb_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tb_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
tb_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=agent "$TB_WAR" --root "$TB_ROOT" "$@"; }

TB_ROOT="$TB_TMP/repo"
mkdir -p "$TB_ROOT"
git -C "$TB_ROOT" init -q .
tb_war init >/dev/null 2>&1
TB_LONG=$(printf 'A sentence of the description that an agent only needs when it takes this one. %.0s' $(seq 1 20))
TB_N=6
for n in $(seq 1 $TB_N); do
    tb_war create "Warrant $n" --body "$TB_LONG MARKER-$n-BODY" \
        --item "item one of $n MARKER-$n-ITEM" --item "item two of $n" --item "item three of $n" >/dev/null 2>&1
done
TB_TEXT=$(( ${#TB_LONG} + 200 ))

# --- prime: one line per Warrant ------------------------------------------------
TB_PRIME=$(tb_war view prime 2>&1)
TB_PRIME_B=$(printf '%s' "$TB_PRIME" | wc -c)
TB_PRIME_MAX=$(( 600 + 200 * TB_N ))
if [[ $TB_PRIME_B -le $TB_PRIME_MAX ]] && ! grep -q 'MARKER-' <<<"$TB_PRIME" \
    && [[ $(grep -c '^- t-' <<<"$TB_PRIME") -eq $TB_N ]]; then
    tb_ok "prime is one line a Warrant" "$TB_PRIME_B bytes for $TB_N (budget $TB_PRIME_MAX); no Warrant's text"
else
    tb_fail "prime is one line a Warrant" "$TB_PRIME_B bytes (budget $TB_PRIME_MAX); markers: $(grep -o 'MARKER-[0-9]*-[A-Z]*' <<<"$TB_PRIME" | tr '\n' ' ')"
fi

# --- ready: one line per Warrant, no item text ----------------------------------
TB_READY=$(tb_war view ready 2>&1)
if [[ $(grep -c '`war start t-' <<<"$TB_READY") -eq $TB_N ]] && ! grep -q 'MARKER-' <<<"$TB_READY"; then
    tb_ok "ready is one line a Warrant" "$(printf '%s' "$TB_READY" | wc -c) bytes; no item text"
else
    tb_fail "ready is one line a Warrant" "$(tr '\n' '|' <<<"$TB_READY" | head -c 400)"
fi

# --- start: only the one taken ----------------------------------------------------
TB_START=$(tb_war start 2>&1)
TB_START_B=$(printf '%s' "$TB_START" | wc -c)
TB_TAKEN=$(grep -o '^# t-[0-9a-f]*' <<<"$TB_START" | head -1 | cut -c3-)
TB_OWN=$(grep -o 'MARKER-[0-9]*-BODY' <<<"$TB_START" | head -1 | cut -d- -f2)
TB_OTHERS=$(grep -o 'MARKER-[0-9]*-[A-Z]*' <<<"$TB_START" | cut -d- -f2 | sort -u | wc -l)
if [[ -n $TB_TAKEN && -n $TB_OWN ]] && grep -q "MARKER-$TB_OWN-ITEM" <<<"$TB_START" \
    && [[ $TB_OTHERS -eq 1 && $TB_START_B -le $(( TB_TEXT + 700 )) ]] \
    && grep -q "war done $TB_TAKEN --note" <<<"$TB_START"; then
    tb_ok "start prints only the one taken" "$TB_TAKEN, $TB_START_B bytes (its text about $TB_TEXT); no other Warrant's"
else
    tb_fail "start prints only the one taken" "$TB_START_B bytes; Warrants named: $TB_OTHERS; taken '$TB_TAKEN'"
fi

# --- done with no id: closes the one held, then holds nothing ----------------------
TB_DONE=$(tb_war --json done --note "finished" 2>/dev/null)
TB_STATE=$(python3 -c 'import json,sys; v=json.loads(sys.argv[1]); r=v["result"]; print(r["ticket"], r["ticket_state"], r["progress"]["done"])' "$TB_DONE" 2>&1)
TB_AGAIN=$(tb_war --json done --note "again" 2>/dev/null)
TB_RULE=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["diagnostics"][0]["rule"])' "$TB_AGAIN" 2>&1)
if [[ $TB_STATE == "$TB_TAKEN done 3" && $TB_RULE == "ticket.nothing-held" ]]; then
    tb_ok "done closes what is held" "$TB_TAKEN: 3 items in one call; a second done is ticket.nothing-held"
else
    tb_fail "done closes what is held" "done: '$TB_STATE'; again: '$TB_RULE'"
fi

rm -rf "$TB_TMP"
