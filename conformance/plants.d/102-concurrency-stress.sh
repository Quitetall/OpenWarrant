# shellcheck shell=bash
# M11 — the concurrency stress plant (t-35e20).
#
# One scratch program (ST) with 30 worktrees of its clone (the main checkout
# and 29 linked ones), plus 2 more clones; all three clones share one bare
# remote, and `[claims] remote = "origin"`. 100 items across 10 tickets.
# One `war claim` loop per checkout (32, background jobs), each reading
# `war ready` and claiming until nothing it has not tried is left.
#
# Accepted: every item is claimed exactly once (the loops' logs, the
# journals of all 32 checkouts and the remote's refs agree); a holder that is
# killed stops renewing, its lease runs out, and another machine reclaims the
# item, journalled from it; under --if-rev, eight agents racing to increment
# one field lose no update (the journal's chain of values has no gap or
# repeat). Throughput is printed: claims per second over the race.
# Refused: every lost race, by name (ticket.claimed-by-other); a claim on the
# killed holder's item while it still heartbeats; and the lost update itself
# without --if-rev (two writers from one read: one increment survives), so
# the chain check is seen to catch one. Beads (`bd`) is not run unless it is
# installed; its absence is said, not passed.

echo "== concurrency: stress, 30 worktrees and 2 clones (M11) =="
ST_ROOT=$(scratch_corpus ST)
[[ -d "${ST_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
ST_TMP=$(mktemp -d)
SCRATCH_CORPORA+=("$ST_TMP")
ST_WAR="$REPO_ROOT/${WAR#./}"
st_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
st_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
st_war() { local at="$1"; shift; env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$ST_WAR" --root "$at" "$@" </dev/null; }
st_commit() { git -C "$1" add -A >/dev/null 2>&1 && git -C "$1" -c user.email=plant@invalid -c user.name=plant commit -qm "$2" >/dev/null 2>&1; }

# ---- setup ----------------------------------------------------------------
printf '\n[claims]\nremote = "origin"\n' >> "$ST_ROOT/openwarrant.toml"
for t in 0 1 2 3 4 5 6 7 8 9; do
    st_args=()
    for i in 0 1 2 3 4 5 6 7 8 9; do st_args+=(--item "Item $t.$i"); done
    st_war "$ST_ROOT" create "Stress $t" "${st_args[@]}" >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: war create in %s\n' "$ST_ROOT" >&2; exit 9; }
done
ST_V=$(st_war "$ST_ROOT" --json create "Victim" --item "Held by a holder that dies" 2>/dev/null \
    | python3 -c 'import json,sys; v=json.load(sys.stdin)["result"]; print(v["id"]+"/"+v["items"][0]["id"])')
ST_K=$(st_war "$ST_ROOT" --json create "Counter" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])')
st_commit "$ST_ROOT" "100 items, a victim, a counter"
git clone -q --bare "$ST_ROOT" "$ST_TMP/bare.git" && git -C "$ST_ROOT" remote add origin "$ST_TMP/bare.git" \
    && git -C "$ST_ROOT" fetch -q origin \
    || { printf 'PLANT SETUP FAILED: the bare remote\n' >&2; exit 9; }
ST_AT=("$ST_ROOT")
for n in $(seq 2 30); do
    git -C "$ST_ROOT" worktree add -q --detach "$ST_TMP/wt$n" >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: worktree %s\n' "$n" >&2; exit 9; }
    ST_AT+=("$ST_TMP/wt$n")
done
for c in c1 c2; do
    git clone -q "$ST_TMP/bare.git" "$ST_TMP/$c" || { printf 'PLANT SETUP FAILED: clone %s\n' "$c" >&2; exit 9; }
    ST_AT+=("$ST_TMP/$c")
done
ST_ALL=$(st_war "$ST_ROOT" ready 2>/dev/null | awk '$1 ~ /^t-/ && $0 ~ /Item [0-9]/ {print $1}' | sort)
[[ $(wc -l <<<"$ST_ALL") -eq 100 ]] || { printf 'PLANT SETUP FAILED: %s items ready, not 100\n' "$(wc -l <<<"$ST_ALL")" >&2; exit 9; }
printf '%s\n' "$ST_ALL" > "$ST_TMP/pool"

# ---- the race -------------------------------------------------------------
# st_agent <checkout> <name>: ready, pick one of the first four untried
# pool items, claim; again until none is left. Logs won, lost (with the
# refusal) and anything else.
st_agent() {
    local at="$1" me="$2" log="$ST_TMP/log/$2" tried=" " next err rc
    : >"$log.won"; : >"$log.lost"; : >"$log.bad"
    while :; do
        next=$(st_war "$at" ready --as "$me" 2>/dev/null | awk '$1 ~ /^t-/ {print $1}' \
            | grep -F -x -f "$ST_TMP/pool" | while read -r id; do
                [[ "$tried" == *" $id "* ]] || echo "$id"
            done | head -4 | shuf -n 1)
        [[ -z "$next" ]] && break
        tried="$tried$next "
        err=$(st_war "$at" claim "$next" --as "$me" 2>&1 >/dev/null); rc=$?
        if [[ $rc -eq 0 ]]; then
            echo "$next" >>"$log.won"
        elif [[ $rc -eq 2 ]] && grep -q '^refused (ticket.claimed-by-other): ' <<<"$err"; then
            echo "$next" >>"$log.lost"
        else
            echo "$next exit $rc: $err" >>"$log.bad"
        fi
    done
}
mkdir -p "$ST_TMP/log"
ST_PIDS=()
ST_T0=$(date +%s%N)
for k in "${!ST_AT[@]}"; do
    st_agent "${ST_AT[$k]}" "agent-$k" &
    ST_PIDS+=($!)
done
for pid in "${ST_PIDS[@]}"; do wait "$pid"; done
ST_T1=$(date +%s%N)
ST_MS=$(( (ST_T1 - ST_T0) / 1000000 ))
ST_WON=$(cat "$ST_TMP"/log/*.won | sort)
ST_NWON=$(wc -l <<<"$ST_WON")
ST_NUNIQ=$(sort -u <<<"$ST_WON" | wc -l)
ST_NLOST=$(cat "$ST_TMP"/log/*.lost | wc -l)
ST_BAD=$(cat "$ST_TMP"/log/*.bad)
# The journals of every checkout, and the remote's refs, by item.
for at in "${ST_AT[@]}"; do printf '%s\n' "$at"/docs/tickets/*/journal.jsonl; done >"$ST_TMP/journals"
ST_JCOUNT=$(python3 - "$ST_TMP/pool" "$ST_TMP/journals" <<'PY'
import json, sys
pool = set(open(sys.argv[1]).read().split())
seen = {}
for path in open(sys.argv[2]).read().split("\n"):
    if not path:
        continue
    for line in open(path):
        e = json.loads(line)
        if e["type"] in ("ticket.claimed", "ticket.claim_reclaimed", "ticket.claim_stolen"):
            t = json.loads(e["payload"])["target"]
            if t in pool:
                seen[t] = seen.get(t, 0) + 1
print(len(seen), sum(seen.values()), sorted(t for t, n in seen.items() if n != 1)[:3])
PY
)
ST_REFS=$(git -C "$ST_TMP/bare.git" for-each-ref --format='%(refname:lstrip=3) %(contents)' refs/openwarrant/claims/ \
    | python3 -c '
import json, sys
for line in sys.stdin:
    name, _, body = line.partition(" ")
    if body.strip():
        c = json.loads(body)
        print(c["ticket"] + "/" + c["item"], c["actor"])
' | sort)
ST_LOGS=$(for f in "$ST_TMP"/log/*.won; do a=$(basename "$f" .won); sed "s/\$/ $a/" "$f"; done | sort)
ST_REFS_POOL=$(grep -F -f "$ST_TMP/pool" <<<"$ST_REFS")
if [[ $ST_NWON -eq 100 && $ST_NUNIQ -eq 100 && "$ST_JCOUNT" == "100 100 []" && "$ST_REFS_POOL" == "$ST_LOGS" ]]; then
    st_ok "zero double claims" "100 items, each claimed once: ${#ST_AT[@]} loops' logs, every checkout's journal and the remote's 100 refs agree"
else
    st_fail "zero double claims" "won $ST_NWON ($ST_NUNIQ distinct); journals (items, events, off) $ST_JCOUNT; refs match logs: $([[ "$ST_REFS_POOL" == "$ST_LOGS" ]] && echo yes || echo no)"
fi
if [[ -z "$ST_BAD" && $ST_NLOST -gt 0 ]]; then
    st_ok "every lost race refused by name" "$ST_NLOST lost races, each ticket.claimed-by-other naming the holder; nothing else"
else
    st_fail "every lost race refused by name" "$ST_NLOST named; other outcomes: $(head -3 <<<"$ST_BAD")"
fi
ST_RATE=$(python3 -c "print(f'{100 / ($ST_MS / 1000):.1f}')")
ST_ARATE=$(python3 -c "print(f'{($ST_NWON + $ST_NLOST) / ($ST_MS / 1000):.1f}')")
printf 'info  %-34s %s\n' "throughput" "100 claims in $ST_MS ms across ${#ST_AT[@]} loops (30 worktrees + 2 clones, remote on): $ST_RATE claims/s; $((ST_NWON + ST_NLOST)) claim attempts, $ST_ARATE attempts/s"
if command -v bd >/dev/null 2>&1; then
    printf 'info  %-34s %s\n' "Beads baseline" "bd is installed ($(bd --version 2>/dev/null | head -1)); a like-for-like bd race is not scripted here, so no Beads number is claimed"
else
    printf 'UNKNOWN %-32s %s\n' "Beads baseline" "bd is not installed on this machine; no Beads number to set beside $ST_RATE claims/s"
fi

# ---- a killed holder --------------------------------------------------------
# The victim's checkout leases for 5 s; a loop renews it every second until
# it is killed. Refused while it lives; reclaimed from the other machine once
# its lease has run out.
ST_W1="${ST_AT[1]}"
printf '\n[tickets]\nclaim_lease_minutes = 0.0834\n' >> "$ST_W1/openwarrant.toml"
st_war "$ST_W1" claim "$ST_V" --as victim >/dev/null 2>&1; ST_S=$?
( while :; do st_war "$ST_W1" heartbeat --as victim >/dev/null 2>&1; sleep 1; done ) &
ST_HB=$!
sleep 6
ST_ERR=$(st_war "$ST_TMP/c1" claim "$ST_V" --as heir 2>&1 >/dev/null); ST_S2=$?
kill -9 "$ST_HB" 2>/dev/null; wait "$ST_HB" 2>/dev/null
sleep 6
ST_OUT=$(st_war "$ST_TMP/c1" claim "$ST_V" --as heir 2>&1); ST_S3=$?
ST_VREF=$(git -C "$ST_TMP/bare.git" log -1 --format=%B "refs/openwarrant/claims/${ST_V/\//--}" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["actor"])' 2>/dev/null)
ST_VJ=$(grep '"ticket.claim_reclaimed"' "$ST_TMP/c1/docs/tickets/${ST_V%%/*}/journal.jsonl")
if [[ $ST_S -eq 0 && $ST_S2 -eq 2 && $ST_S3 -eq 0 && "$ST_VREF" == heir ]] && grep -q 'claimed on the remote .origin. by victim' <<<"$ST_ERR" \
    && grep -q 'reclaimed from victim' <<<"$ST_OUT" && grep -q 'from\\":\\"victim' <<<"$ST_VJ"; then
    st_ok "a killed holder's item is reclaimed" "6 s in, heartbeating: heir refused by name; killed, its 5 s lease ran out: heir reclaimed it across the remote, journalled from victim"
else
    st_fail "a killed holder's item is reclaimed" "claim $ST_S; while alive $ST_S2 ($ST_ERR); after $ST_S3 ($ST_OUT); remote holder '$ST_VREF'"
fi

# ---- no lost update under --if-rev ------------------------------------------
st_read() { st_war "$ST_ROOT" show "$ST_K" --json 2>/dev/null | python3 -c 'import json,sys; v=json.load(sys.stdin)["result"]; print(v["revision"], v["ticket"]["priority"])'; }
st_edits() { python3 -c '
import json, sys
for line in open(sys.argv[1]):
    e = json.loads(line)
    if e["type"] == "ticket.edited":
        p = json.loads(e["payload"]).get("priority")
        if p is not None:
            print(p)
' "$ST_ROOT/docs/tickets/$ST_K/journal.jsonl"; }
# The lost update, made on purpose: two writers increment from one read.
read -r _ ST_A0 < <(st_read)
st_war "$ST_ROOT" edit "$ST_K" --priority $(( (ST_A0 + 1) % 5 )) --as u1 >/dev/null 2>&1; ST_S=$?
st_war "$ST_ROOT" edit "$ST_K" --priority $(( (ST_A0 + 1) % 5 )) --as u2 >/dev/null 2>&1; ST_S1=$?
read -r _ ST_A1 < <(st_read)
# The same with --if-rev: the second writer is refused, rereads, and lands.
read -r ST_R0 ST_B0 < <(st_read)
st_war "$ST_ROOT" edit "$ST_K" --priority $(( (ST_B0 + 1) % 5 )) --if-rev "$ST_R0" --as g1 >/dev/null 2>&1
ST_ERR=$(st_war "$ST_ROOT" edit "$ST_K" --priority $(( (ST_B0 + 1) % 5 )) --if-rev "$ST_R0" --as g2 2>&1 >/dev/null); ST_S2=$?
read -r ST_R1 ST_B1 < <(st_read)
st_war "$ST_ROOT" edit "$ST_K" --priority $(( (ST_B1 + 1) % 5 )) --if-rev "$ST_R1" --as g2 >/dev/null 2>&1
read -r _ ST_B2 < <(st_read)
if [[ $ST_S -eq 0 && $ST_S1 -eq 0 && "$ST_A1" == $(( (ST_A0 + 1) % 5 )) && $ST_S2 -eq 2 && "$ST_B2" == $(( (ST_B0 + 2) % 5 )) ]] \
    && grep -q 'warrant.stale-revision' <<<"$ST_ERR"; then
    st_ok "the lost update, and its guard" "unguarded, two increments from one read: $ST_A0 -> $ST_A1, one lost; with --if-rev the second was refused by name, reread and landed: $ST_B0 -> $ST_B2"
else
    st_fail "the lost update, and its guard" "unguarded exits $ST_S/$ST_S1, $ST_A0 -> $ST_A1; guarded second $ST_S2 ($ST_ERR), $ST_B0 -> $ST_B2"
fi
# Eight agents, four increments each, retrying on a stale revision.
st_incr() {
    local me="$1" done=0 tries=0 r p
    while [[ $done -lt 4 && $tries -lt 200 ]]; do
        tries=$((tries + 1))
        read -r r p < <(st_read)
        [[ -n "$r" ]] || continue
        if st_war "$ST_ROOT" edit "$ST_K" --priority $(( (p + 1) % 5 )) --if-rev "$r" --as "$me" >/dev/null 2>"$ST_TMP/log/$me.err"; then
            done=$((done + 1))
        elif ! grep -q 'warrant.stale-revision' "$ST_TMP/log/$me.err"; then
            echo "$me: $(cat "$ST_TMP/log/$me.err")" >>"$ST_TMP/log/incr.bad"
        fi
    done
    echo "$done" >"$ST_TMP/log/$me.incr"
}
ST_E0=$(st_edits | wc -l)
read -r _ ST_PS < <(st_read)
ST_PIDS=()
for n in 1 2 3 4 5 6 7 8; do st_incr "inc-$n" & ST_PIDS+=($!); done
for pid in "${ST_PIDS[@]}"; do wait "$pid"; done
ST_DONE=$(awk '{s += $1} END {print s}' "$ST_TMP"/log/inc-*.incr)
read -r _ ST_PE < <(st_read)
ST_CHAIN=$(st_edits | tail -n +$((ST_E0 + 1)) | python3 -c "
import sys
vals = [int(x) for x in sys.stdin.read().split()]
prev, bad = $ST_PS, 0
for v in vals:
    bad += v != (prev + 1) % 5
    prev = v
print(len(vals), bad)
")
if [[ "$ST_DONE" == 32 && "$ST_CHAIN" == "32 0" && "$ST_PE" == $(( (ST_PS + 32) % 5 )) && ! -s "$ST_TMP/log/incr.bad" ]]; then
    st_ok "no lost update under --if-rev" "8 agents x 4 increments: 32 writes, each journalled value one past the last, no gap or repeat"
else
    st_fail "no lost update under --if-rev" "writes $ST_DONE; chain (writes, breaks) $ST_CHAIN; value $ST_PE from $ST_PS; $(head -2 "$ST_TMP/log/incr.bad" 2>/dev/null)"
fi
