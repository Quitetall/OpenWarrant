# shellcheck shell=bash
# M11 — concurrency at Beads' level or better (t-35e20).
#
# One scratch program (CC) with a second worktree of the same clone.
#
# Shared claims. Accepted: a claim taken in one worktree lands under git's
# common directory and the other worktree sees it; an item no one holds is
# claimed from the second worktree; a claim from before M11, lying in a
# worktree's own .openwarrant/state/claims/, is honoured from every worktree
# and released by its holder. Refused, by name (ticket.claimed-by-other): the
# 2026-10-06 reproduction, the same item claimed from the second worktree; a
# second claim in the same worktree; and a claim on the item the pre-M11 lock
# holds.

echo "== concurrency: shared claims (M11) =="
CC_ROOT=$(scratch_corpus CC)
[[ -d "${CC_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
CC_TMP=$(mktemp -d)
SCRATCH_CORPORA+=("$CC_TMP")
CC_WAR="$REPO_ROOT/${WAR#./}"
cc_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# cc_war <checkout> <war args...>: no terminal, no inherited actor.
cc_war() { local at="$1"; shift; env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$CC_WAR" --root "$at" "$@" </dev/null; }
cc_json() { cc_war "$@" --json 2>/dev/null; }
cc_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
cc_commit() { git -C "$1" add -A >/dev/null 2>&1 && git -C "$1" -c user.email=plant@invalid -c user.name=plant commit -qm "$2" >/dev/null 2>&1; }

CC_OUT=$(cc_json "$CC_ROOT" create "Shared work" --item "First" --item "Second" --item "Third")
CC_T=$(cc_field "$CC_OUT" 'v["result"]["id"]')
CC_I1=$(cc_field "$CC_OUT" 'v["result"]["items"][0]["id"]')
CC_I2=$(cc_field "$CC_OUT" 'v["result"]["items"][1]["id"]')
CC_I3=$(cc_field "$CC_OUT" 'v["result"]["items"][2]["id"]')
cc_commit "$CC_ROOT" "a ticket"
CC_WT="$CC_TMP/wt2"
git -C "$CC_ROOT" worktree add -q "$CC_WT" >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: git worktree add in %s\n' "$CC_ROOT" >&2; exit 9; }
CC_COMMON=$(git -C "$CC_WT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)

# The reproduction: alice in the first worktree, bob in the second.
cc_war "$CC_ROOT" claim "$CC_T/$CC_I1" --as alice >/dev/null 2>&1; CC_S1=$?
CC_ERR=$(cc_war "$CC_WT" claim "$CC_T/$CC_I1" --as bob 2>&1 >/dev/null); CC_S2=$?
CC_LOCK="$CC_COMMON/openwarrant/claims/$CC_T--$CC_I1.lock"
CC_HELD=0; [[ -f "$CC_LOCK" ]] && CC_HELD=1
if [[ $CC_S1 -eq 0 && $CC_S2 -eq 2 && $CC_HELD -eq 1 ]] && grep -q 'ticket.claimed-by-other' <<<"$CC_ERR" \
    && grep -q "claimed by alice since" <<<"$CC_ERR" \
    && [[ ! -e "$CC_ROOT/.openwarrant/state/claims/$CC_T--$CC_I1.lock" ]]; then
    cc_ok "two worktrees, one claim" "bob in the second worktree refused by name; alice's lock under $(basename "$CC_COMMON")/openwarrant/claims"
else
    cc_fail "two worktrees, one claim" "alice $CC_S1, bob $CC_S2 ($CC_ERR), lock at common dir: $CC_HELD"
fi

# Still refused within one worktree.
CC_ERR=$(cc_war "$CC_ROOT" claim "$CC_T/$CC_I1" --as carol 2>&1 >/dev/null); CC_S=$?
if [[ $CC_S -eq 2 ]] && grep -q 'ticket.claimed-by-other' <<<"$CC_ERR" && grep -q 'claimed by alice' <<<"$CC_ERR"; then
    cc_ok "one worktree, one claim" "carol refused by name in alice's worktree"
else
    cc_fail "one worktree, one claim" "exit $CC_S: $CC_ERR"
fi

# The accepting half: a free item from the second worktree, seen from the first.
cc_war "$CC_WT" claim "$CC_T/$CC_I2" --as bob >/dev/null 2>&1; CC_S=$?
CC_SEEN=$(cc_field "$(cc_json "$CC_ROOT" tickets)" '",".join(sorted(c["actor"]+"@"+str(c.get("item")) for r in v["result"]["tickets"] for c in r["claims"]))')
if [[ $CC_S -eq 0 && "$CC_SEEN" == "alice@$CC_I1,bob@$CC_I2" ]]; then
    cc_ok "a claim is seen from every worktree" "bob took $CC_I2 in the second; the first lists $CC_SEEN"
else
    cc_fail "a claim is seen from every worktree" "claim exit $CC_S; listed '$CC_SEEN'"
fi

# A claim from before M11, in the second worktree's own directory.
mkdir -p "$CC_WT/.openwarrant/state/claims"
printf '{"schema":"oh.war/ticket-claim/v1","claim":"01a10000-0000-7000-8000-000000000001","ticket":"%s","item":"%s","actor":"olddave","since":"2026-10-06T00:00:00Z","since_unix":%s}\n' \
    "$CC_T" "$CC_I3" "$(date +%s)" > "$CC_WT/.openwarrant/state/claims/$CC_T--$CC_I3.lock"
CC_ERR=$(cc_war "$CC_ROOT" claim "$CC_T/$CC_I3" --as erin 2>&1 >/dev/null); CC_S=$?
cc_war "$CC_WT" release "$CC_T/$CC_I3" --as olddave >/dev/null 2>&1; CC_S2=$?
CC_GONE=0; [[ -e "$CC_WT/.openwarrant/state/claims/$CC_T--$CC_I3.lock" ]] || CC_GONE=1
cc_war "$CC_ROOT" claim "$CC_T/$CC_I3" --as erin >/dev/null 2>&1; CC_S3=$?
if [[ $CC_S -eq 2 && $CC_S2 -eq 0 && $CC_GONE -eq 1 && $CC_S3 -eq 0 ]] && grep -q 'claimed by olddave' <<<"$CC_ERR" \
    && [[ -f "$CC_COMMON/openwarrant/claims/$CC_T--$CC_I3.lock" ]]; then
    cc_ok "a pre-M11 claim is honoured" "erin refused (olddave's lock in the other worktree), then olddave released it and erin's claim went to the shared directory"
else
    cc_fail "a pre-M11 claim is honoured" "erin $CC_S ($CC_ERR), release $CC_S2, gone $CC_GONE, erin again $CC_S3"
fi

# Leases. A one-minute lease, and a lock set back two minutes by hand (its
# claim's times and its modification time) to stand for an agent that stopped
# renewing: no sleeping, so a loaded machine cannot move the result. Accepted: a claim carries lease_until; `war heartbeat`, and
# any war command the holder runs (`war check` here, as $OPENWARRANT_ACTOR),
# renews it; an expired lease is offered by `war ready` and reclaimed by a
# plain `war claim`, journalled with the previous holder. Refused: a claim on
# a live lease (ticket.claimed-by-other, naming when it runs out); the old
# holder's done after the reclaim; and the lapsed state itself, observed
# before each renewal so the renewal is what kept the claim.
echo "== concurrency: leases (M11) =="
cc_config() { git -C "$CC_ROOT" checkout -q -- openwarrant.toml && printf '\n[tickets]\n%s\n' "$1" >> "$CC_ROOT/openwarrant.toml"; }
cc_config 'claim_lease_minutes = 1'
CC_OUT=$(cc_json "$CC_ROOT" create "Leases" --item "Leased" --item "Stolen")
CC_L=$(cc_field "$CC_OUT" 'v["result"]["id"]')
CC_LA=$(cc_field "$CC_OUT" 'v["result"]["items"][0]["id"]')
CC_LB=$(cc_field "$CC_OUT" 'v["result"]["items"][1]["id"]')
CC_LLOCK="$CC_COMMON/openwarrant/claims/$CC_L--$CC_LA.lock"
# cc_lapse: the lock as an agent that took it, and last renewed it, two
# minutes ago left it.
cc_lapse() {
    python3 - "$CC_LLOCK" <<'PY2'
import json, sys, time
p = sys.argv[1]
c = json.load(open(p))
back = int(time.time()) - 120 - c["since_unix"]
for k in ("since_unix", "lease_until_unix"):
    c[k] += back
c["since"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(c["since_unix"]))
c["lease_until"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(c["lease_until_unix"]))
open(p, "w").write(json.dumps(c, indent=2) + "\n")
PY2
    touch -d "@$(( $(date +%s) - 120 ))" "$CC_LLOCK"
}
# cc_offered: whether `war ready` (as ivy) offers the item, and why.
cc_offered() { cc_field "$(cc_json "$CC_ROOT" ready --as ivy)" '",".join(("expired:"+r["expired_claim"]["actor"]) if r.get("expired_claim") else "free" for r in v["result"]["ready"] if r.get("item")=="'"$CC_LA"'") or "held"'; }
CC_C=$(cc_json "$CC_ROOT" claim "$CC_L/$CC_LA" --as hank)
CC_LEASE=$(cc_field "$CC_C" 'v["result"]["claim"]["lease_until_unix"] - v["result"]["claim"]["since_unix"]')
CC_ERR=$(cc_war "$CC_ROOT" claim "$CC_L/$CC_LA" --as ivy 2>&1 >/dev/null); CC_S=$?
if [[ "$CC_LEASE" == 60 && $CC_S -eq 2 ]] && grep -q 'claimed by hank' <<<"$CC_ERR" && grep -q 'lease runs out in' <<<"$CC_ERR"; then
    cc_ok "a claim carries its lease" "lease_until 60 s after since; ivy refused by name while it runs"
else
    cc_fail "a claim carries its lease" "lease $CC_LEASE s; ivy exit $CC_S: $CC_ERR"
fi
cc_lapse; CC_BEFORE=$(cc_offered)
cc_war "$CC_ROOT" heartbeat --as hank >/dev/null 2>&1; CC_S=$?
CC_AFTER=$(cc_offered)
CC_ERR=$(cc_war "$CC_ROOT" claim "$CC_L/$CC_LA" --as ivy 2>&1 >/dev/null); CC_S2=$?
if [[ "$CC_BEFORE" == "expired:hank" && $CC_S -eq 0 && "$CC_AFTER" == "held" && $CC_S2 -eq 2 ]] && grep -q 'claimed by hank' <<<"$CC_ERR"; then
    cc_ok "war heartbeat renews the lease" "lapsed: offered as expired; after hank's heartbeat: held, ivy refused"
else
    cc_fail "war heartbeat renews the lease" "before '$CC_BEFORE', heartbeat $CC_S, after '$CC_AFTER', ivy $CC_S2: $CC_ERR"
fi
cc_lapse; CC_BEFORE=$(cc_offered)
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=hank "$CC_WAR" --root "$CC_ROOT" check </dev/null >/dev/null 2>&1
CC_AFTER=$(cc_offered)
CC_ERR=$(cc_war "$CC_ROOT" claim "$CC_L/$CC_LA" --as ivy 2>&1 >/dev/null); CC_S2=$?
if [[ "$CC_BEFORE" == "expired:hank" && "$CC_AFTER" == "held" && $CC_S2 -eq 2 ]]; then
    cc_ok "any war command renews it" "lapsed, then hank ran \`war check\`: held, ivy refused"
else
    cc_fail "any war command renews it" "before '$CC_BEFORE', after '$CC_AFTER', ivy $CC_S2: $CC_ERR"
fi
cc_lapse
CC_OUT=$(cc_war "$CC_ROOT" claim "$CC_L/$CC_LA" --as ivy 2>&1); CC_S=$?
CC_ERR=$(cc_war "$CC_ROOT" done "$CC_L/$CC_LA" --as hank 2>&1 >/dev/null); CC_S2=$?
CC_J="$CC_ROOT/docs/tickets/$CC_L/journal.jsonl"
if [[ $CC_S -eq 0 && $CC_S2 -eq 2 ]] && grep -q 'reclaimed from hank' <<<"$CC_OUT" \
    && grep -q '"ticket.claim_reclaimed".*from\\":\\"hank.*from_lease_until' "$CC_J" \
    && grep -q 'claimed by ivy' <<<"$CC_ERR"; then
    cc_ok "an expired lease is reclaimed" "ivy took it with a plain claim, journalled from hank; hank's done refused"
else
    cc_fail "an expired lease is reclaimed" "ivy $CC_S ($CC_OUT); hank's done $CC_S2 ($CC_ERR)"
fi
# --steal still takes a claim past the TTL whose lease is live.
cc_config $'claim_lease_minutes = 30\nclaim_ttl_minutes = 0'
cc_war "$CC_ROOT" claim "$CC_L/$CC_LB" --as jill >/dev/null 2>&1
sleep 1
CC_ERR=$(cc_war "$CC_ROOT" claim "$CC_L/$CC_LB" --as kim 2>&1 >/dev/null); CC_S=$?
cc_war "$CC_ROOT" claim "$CC_L/$CC_LB" --as kim --steal >/dev/null 2>&1; CC_S2=$?
if [[ $CC_S -eq 2 && $CC_S2 -eq 0 ]] && grep -q -- '--steal. takes it' <<<"$CC_ERR" \
    && grep -q '"ticket.claim_stolen".*from\\":\\"jill' "$CC_J"; then
    cc_ok "--steal is for a live lease" "past the TTL with 30 min of lease left: plain claim refused, --steal took it from jill"
else
    cc_fail "--steal is for a live lease" "plain $CC_S ($CC_ERR), steal $CC_S2"
fi
git -C "$CC_ROOT" checkout -q -- openwarrant.toml

# Compare-and-set. `war show --json` gives the ticket's revision and each
# item's (the same digests `war model` reports). Accepted: four agents race
# `war edit --priority` from one read with --if-rev, and exactly one write
# lands; done, note, add and release at the current revision go through, and
# without --if-rev everything behaves as before (the same race, unguarded,
# lets every write through and only the last survives: three lost updates).
# Refused, warrant.stale-revision naming the current revision, nothing
# written: the three race losers; done after the item's line moved; note,
# add and release at a stale revision; and war_note over MCP with a stale
# if_rev.
echo "== concurrency: compare-and-set (M11) =="
CC_OUT=$(cc_json "$CC_ROOT" create "CAS" --item "Guarded" --item "Other")
CC_K=$(cc_field "$CC_OUT" 'v["result"]["id"]')
CC_KA=$(cc_field "$CC_OUT" 'v["result"]["items"][0]["id"]')
cc_rev() { cc_field "$(cc_json "$CC_ROOT" show "$CC_K")" "$1"; }
CC_R0=$(cc_rev 'v["result"]["revision"]')
CC_MODEL=$(cc_field "$(cc_json "$CC_ROOT" model)" '",".join(r["revision"] for r in v["result"]["records"] if r["id"] in ("'"$CC_K"'", "'"$CC_K/$CC_KA"'"))')
CC_SHOWN=$(cc_rev '",".join([v["result"]["revision"]] + [i["revision"] for i in v["result"]["items"] if i["id"]=="'"$CC_KA"'"])')
CC_EDITS0=$(grep -c '"ticket.edited"' "$CC_ROOT/docs/tickets/$CC_K/journal.jsonl")
CC_PIDS=(); CC_DIR=$(mktemp -d -p "$CC_TMP")
for p in 0 1 3 4; do
    ( cc_war "$CC_ROOT" edit "$CC_K" --priority "$p" --if-rev "$CC_R0" --as "racer-$p" >"$CC_DIR/out.$p" 2>"$CC_DIR/err.$p"; echo $? >"$CC_DIR/rc.$p" ) &
    CC_PIDS+=($!)
done
for pid in "${CC_PIDS[@]}"; do wait "$pid"; done
CC_WON=""; CC_STALE=0
for p in 0 1 3 4; do
    if [[ "$(cat "$CC_DIR/rc.$p")" == 0 ]]; then CC_WON="$CC_WON$p"; fi
    if grep -q 'warrant.stale-revision' "$CC_DIR/err.$p" && grep -q 'and it is now sha256:' "$CC_DIR/err.$p"; then CC_STALE=$((CC_STALE + 1)); fi
done
CC_PRIO=$(cc_field "$(cc_json "$CC_ROOT" show "$CC_K")" 'v["result"]["ticket"]["priority"]')
CC_EDITS=$(( $(grep -c '"ticket.edited"' "$CC_ROOT/docs/tickets/$CC_K/journal.jsonl") - CC_EDITS0 ))
if [[ "$CC_SHOWN" == "$CC_MODEL" && ${#CC_WON} -eq 1 && $CC_STALE -eq 3 && "$CC_PRIO" == "$CC_WON" && $CC_EDITS -eq 1 ]]; then
    cc_ok "--if-rev: one write of four lands" "show's revisions are war model's; racer-$CC_WON won; 3 refused warrant.stale-revision naming the current revision; one ticket.edited"
else
    cc_fail "--if-rev: one write of four lands" "show '$CC_SHOWN' model '$CC_MODEL' won '$CC_WON' stale $CC_STALE priority $CC_PRIO edits $CC_EDITS: $(cat "$CC_DIR"/err.* | head -2)"
fi
# The same race without --if-rev: every write exits 0, and the last one wins.
CC_PIDS=(); CC_EDITS0=$(grep -c '"ticket.edited"' "$CC_ROOT/docs/tickets/$CC_K/journal.jsonl")
CC_FROM=$(cc_field "$(cc_json "$CC_ROOT" show "$CC_K")" 'v["result"]["ticket"]["priority"]')
for p in 0 1 3 4; do
    [[ "$p" == "$CC_FROM" ]] && continue
    ( cc_war "$CC_ROOT" edit "$CC_K" --priority "$p" --as "racer-$p" >/dev/null 2>&1; echo $? >"$CC_DIR/rc2.$p" ) &
    CC_PIDS+=($!)
done
for pid in "${CC_PIDS[@]}"; do wait "$pid"; done
CC_OK2=$(cat "$CC_DIR"/rc2.* | grep -c '^0$')
CC_EDITS=$(( $(grep -c '"ticket.edited"' "$CC_ROOT/docs/tickets/$CC_K/journal.jsonl") - CC_EDITS0 ))
if [[ $CC_OK2 -eq 3 && $CC_EDITS -ge 2 ]]; then
    cc_ok "without --if-rev: as before" "three unguarded edits all exit 0 ($CC_EDITS journalled); one priority survives, the others lost"
else
    cc_fail "without --if-rev: as before" "$CC_OK2 of 3 exited 0; $CC_EDITS edits journalled"
fi
# Item revisions: done after the line moved is refused; at the new revision it lands.
cc_war "$CC_ROOT" claim "$CC_K/$CC_KA" --as lena >/dev/null 2>&1
CC_IR=$(cc_rev '[i["revision"] for i in v["result"]["items"] if i["id"]=="'"$CC_KA"'"][0]')
CC_KR=$(cc_rev 'v["result"]["revision"]')
sed -i "s/^- \[ \] Guarded ($CC_KA)/- [ ] Guarded, reworded ($CC_KA)/" "$CC_ROOT/docs/tickets/$CC_K/atoms/15-checklist.md"
CC_ERR=$(cc_war "$CC_ROOT" done "$CC_K/$CC_KA" --as lena --if-rev "$CC_IR" 2>&1 >/dev/null); CC_S=$?
CC_ERR2=$(cc_war "$CC_ROOT" note "$CC_K/$CC_KA" "stale" --as lena --if-rev "$CC_IR" 2>&1 >/dev/null); CC_S2=$?
CC_ERR3=$(cc_war "$CC_ROOT" release "$CC_K/$CC_KA" --as lena --if-rev "$CC_IR" 2>&1 >/dev/null); CC_S3=$?
CC_ERR4=$(cc_war "$CC_ROOT" add "$CC_K" "late" --as lena --if-rev "$CC_R0" 2>&1 >/dev/null); CC_S4=$?
CC_UNTICKED=$(grep -c "^- \[ \] Guarded, reworded ($CC_KA)$" "$CC_ROOT/docs/tickets/$CC_K/atoms/15-checklist.md")
CC_HELD=0; [[ -f "$CC_COMMON/openwarrant/claims/$CC_K--$CC_KA.lock" ]] && CC_HELD=1
CC_IR2=$(cc_rev '[i["revision"] for i in v["result"]["items"] if i["id"]=="'"$CC_KA"'"][0]')
cc_war "$CC_ROOT" note "$CC_K/$CC_KA" "current" --as lena --if-rev "$CC_IR2" >/dev/null 2>&1; CC_S5=$?
cc_war "$CC_ROOT" add "$CC_K" "on time" --as lena --if-rev "$CC_KR" >/dev/null 2>&1; CC_S6=$?
cc_war "$CC_ROOT" done "$CC_K/$CC_KA" --as lena --if-rev "$CC_IR2" >/dev/null 2>&1; CC_S7=$?
CC_ALL=0
for e in "$CC_ERR" "$CC_ERR2" "$CC_ERR3" "$CC_ERR4"; do grep -q 'warrant.stale-revision' <<<"$e" && CC_ALL=$((CC_ALL + 1)); done
if [[ $CC_S -eq 2 && $CC_S2 -eq 2 && $CC_S3 -eq 2 && $CC_S4 -eq 2 && $CC_ALL -eq 4 && $CC_UNTICKED -eq 1 && $CC_HELD -eq 1 \
    && $CC_S5 -eq 0 && $CC_S6 -eq 0 && $CC_S7 -eq 0 ]] && grep -q "and it is now $CC_IR2" <<<"$CC_ERR"; then
    cc_ok "stale done/note/add/release" "the line moved: four refused by name (the new revision named), nothing ticked, the claim kept; at the current revision all three wrote"
else
    cc_fail "stale done/note/add/release" "stale: $CC_S/$CC_S2/$CC_S3/$CC_S4 ($CC_ALL named), unticked $CC_UNTICKED held $CC_HELD; current: $CC_S5/$CC_S6/$CC_S7; $CC_ERR"
fi
# Over MCP: war_note with a stale if_rev, then with the current one.
CC_KR=$(cc_rev 'v["result"]["revision"]')
CC_TX=$(mktemp -p "$CC_TMP")
{
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"plant","version":"0"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"war_note","arguments":{"target":"%s","text":"stale","if_rev":"%s","actor":"mcp"}}}\n' "$CC_K" "$CC_R0"
    printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"war_note","arguments":{"target":"%s","text":"fresh","if_rev":"%s","actor":"mcp"}}}\n' "$CC_K" "$CC_KR"
} > "$CC_TX"
CC_MCP=$(cd "$CC_ROOT" && env -u OPENWARRANT_ACTOR python3 "$REPO_ROOT/conformance/fixtures/mcp/drive.py" "$CC_WAR" "$CC_TX" 2>/dev/null)
if grep -q '"id":2.*warrant.stale-revision' <<<"$CC_MCP" && grep -q '"id":3.*"exit_code":0' <<<"$CC_MCP" \
    && grep -q 'mcp:\*\* fresh' "$CC_ROOT/docs/tickets/$CC_K/atoms/10-intent.md" \
    && ! grep -q 'mcp:\*\* stale' "$CC_ROOT/docs/tickets/$CC_K/atoms/10-intent.md"; then
    cc_ok "if_rev over MCP" "war_note: stale refused by name, current written"
else
    cc_fail "if_rev over MCP" "$(head -c 400 <<<"$CC_MCP")"
fi

# Across machines. Two clones of one bare remote stand for two machines.
# Accepted: with [claims] unset, a claim publishes nothing; with [claims]
# remote = "origin", eight agents (four per clone) race for one item and
# exactly one wins, its claim the remote's ref; done deletes the ref, and the
# other clone then claims the item; a claim whose lease ran out on the other
# machine is reclaimed across the remote, journalled from its holder.
# Refused, by name: the seven losers (ticket.claimed-by-other; the clone that
# won locally and lost at the remote names the remote's holder and keeps no
# lock); and a claim when the remote cannot be reached
# (ticket.claim-remote-unreachable), nothing claimed.
echo "== concurrency: claims across machines (M11) =="
CC_X="$CC_TMP/machines"; mkdir -p "$CC_X"
git -C "$CC_ROOT" checkout -q -- openwarrant.toml
CC_OUT=$(cc_json "$CC_ROOT" create "Across machines" --item "Contended" --item "Lapsed")
CC_M=$(cc_field "$CC_OUT" 'v["result"]["id"]')
CC_MA=$(cc_field "$CC_OUT" 'v["result"]["items"][0]["id"]')
CC_MB=$(cc_field "$CC_OUT" 'v["result"]["items"][1]["id"]')
cc_commit "$CC_ROOT" "a ticket for two machines"
git clone -q --bare "$CC_ROOT" "$CC_X/bare.git" && git clone -q "$CC_X/bare.git" "$CC_X/m1" && git clone -q "$CC_X/bare.git" "$CC_X/m2" \
    || { printf 'PLANT SETUP FAILED: clones of %s\n' "$CC_ROOT" >&2; exit 9; }
cc_refs() { git -C "$CC_X/bare.git" for-each-ref --format='%(refname)' refs/openwarrant/; }
# Off by default.
cc_war "$CC_X/m1" claim "$CC_M/$CC_MA" --as solo >/dev/null 2>&1; CC_S=$?
CC_OFF=$(cc_refs)
cc_war "$CC_X/m1" release "$CC_M/$CC_MA" --as solo >/dev/null 2>&1
for m in m1 m2; do printf '\n[claims]\nremote = "origin"\n' >> "$CC_X/$m/openwarrant.toml"; done
cc_claimed() { grep -c '"ticket.claimed"' "$CC_X/$1/docs/tickets/$CC_M/journal.jsonl"; }
CC_J1=$(cc_claimed m1); CC_J2=$(cc_claimed m2)
CC_PIDS=()
for n in 1 2 3 4; do
    for m in m1 m2; do
        ( cc_war "$CC_X/$m" claim "$CC_M/$CC_MA" --as "$m-$n" >/dev/null 2>"$CC_X/err.$m-$n"; echo $? >"$CC_X/rc.$m-$n" ) &
        CC_PIDS+=($!)
    done
done
for pid in "${CC_PIDS[@]}"; do wait "$pid"; done
CC_WIN=""; CC_NAMED=0
for f in "$CC_X"/rc.*; do
    a=${f##*/rc.}
    if [[ "$(cat "$f")" == 0 ]]; then CC_WIN="$CC_WIN $a"
    elif grep -q 'ticket.claimed-by-other' "$CC_X/err.$a"; then CC_NAMED=$((CC_NAMED + 1)); fi
done
CC_WIN=${CC_WIN# }
CC_REF="refs/openwarrant/claims/$CC_M--$CC_MA"
CC_REFACTOR=$(git -C "$CC_X/bare.git" log -1 --format=%B "$CC_REF" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["actor"])' 2>/dev/null)
CC_WM=${CC_WIN%%-*}; CC_LM=m1; [[ "$CC_WM" == m1 ]] && CC_LM=m2
CC_LOSERLOCK=0; [[ -e "$CC_X/$CC_LM/.git/openwarrant/claims/$CC_M--$CC_MA.lock" ]] && CC_LOSERLOCK=1
# One claim journalled, in the winner's clone only.
CC_JW=$(( $(cc_claimed m1) + $(cc_claimed m2) - CC_J1 - CC_J2 ))
CC_JL=$(( $(cc_claimed "$CC_LM") - $([[ "$CC_LM" == m1 ]] && echo "$CC_J1" || echo "$CC_J2") ))
if [[ $CC_S -eq 0 && -z "$CC_OFF" && "$CC_WIN" != *" "* && -n "$CC_WIN" && $CC_NAMED -eq 7 && "$CC_REFACTOR" == "$CC_WIN" \
    && $CC_LOSERLOCK -eq 0 && $CC_JW -eq 1 && $CC_JL -eq 0 ]] \
    && grep -q "claimed on the remote .origin. by $CC_WIN" "$CC_X"/err.*; then
    cc_ok "two machines, one claim" "unset: no ref; set: $CC_WIN of 8 won, its claim the remote's ref; 7 refused by name, the losing clone kept no lock"
else
    cc_fail "two machines, one claim" "off '$CC_OFF' ($CC_S); won '$CC_WIN', named $CC_NAMED, ref actor '$CC_REFACTOR', loser lock $CC_LOSERLOCK, journals $CC_JW/$CC_JL; $(cat "$CC_X"/err.* | head -2)"
fi
# Done deletes the ref; the other machine then claims it.
cc_war "$CC_X/$CC_WM" done "$CC_M/$CC_MA" --as "$CC_WIN" >/dev/null 2>&1; CC_S=$?
CC_AFTER=$(cc_refs)
CC_ERR=$(cc_war "$CC_X/$CC_LM" claim "$CC_M/$CC_MB" --as other 2>&1 >/dev/null); CC_S2=$?
if [[ $CC_S -eq 0 && "$CC_AFTER" != *"$CC_M--$CC_MA"* && $CC_S2 -eq 0 ]]; then
    cc_ok "done gives the item back" "the winner's done deleted $CC_REF; the other machine claimed $CC_MB"
else
    cc_fail "done gives the item back" "done $CC_S, refs '$CC_AFTER', other claim $CC_S2: $CC_ERR"
fi
cc_war "$CC_X/$CC_LM" release "$CC_M/$CC_MB" --as other >/dev/null 2>&1
# A holder on the other machine that stopped two minutes ago: its claim, as
# the remote holds it, with a lease that ran out.
CC_TREE=$(git -C "$CC_X/$CC_WM" hash-object -t tree -w --stdin </dev/null)
CC_MSG=$(python3 -c 'import json,sys,time; t=int(time.time())-120; f=lambda s: time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(s)); print(json.dumps({"schema":"oh.war/ticket-claim/v1","claim":"01a10000-0000-7000-8000-000000000002","ticket":sys.argv[1],"item":sys.argv[2],"actor":"gone","since":f(t),"since_unix":t,"lease_until":f(t+60),"lease_until_unix":t+60}))' "$CC_M" "$CC_MB")
CC_C=$(git -C "$CC_X/$CC_WM" -c user.email=plant@invalid -c user.name=plant commit-tree --no-gpg-sign "$CC_TREE" -m "$CC_MSG")
git -C "$CC_X/$CC_WM" push -q origin "$CC_C:refs/openwarrant/claims/$CC_M--$CC_MB" >/dev/null 2>&1
CC_OUT=$(cc_war "$CC_X/$CC_LM" claim "$CC_M/$CC_MB" --as heir 2>&1); CC_S=$?
CC_NOW=$(git -C "$CC_X/bare.git" log -1 --format=%B "refs/openwarrant/claims/$CC_M--$CC_MB" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["actor"])' 2>/dev/null)
CC_JLINE=$(grep '"ticket.claim_reclaimed"' "$CC_X/$CC_LM/docs/tickets/$CC_M/journal.jsonl")
if [[ $CC_S -eq 0 && "$CC_NOW" == heir ]] && grep -q 'reclaimed from gone' <<<"$CC_OUT" \
    && grep -q 'from\\":\\"gone' <<<"$CC_JLINE" && grep -q 'remote\\":\\"origin:refs/openwarrant/claims/' <<<"$CC_JLINE"; then
    cc_ok "a lapsed lease, reclaimed across" "the remote held gone's claim, its lease out; heir reclaimed it, journalled from gone"
else
    cc_fail "a lapsed lease, reclaimed across" "claim $CC_S ($CC_OUT); remote holder '$CC_NOW'"
fi
cc_war "$CC_X/$CC_LM" release "$CC_M/$CC_MB" --as heir >/dev/null 2>&1
# The remote unreachable: refused, nothing claimed here either.
git -C "$CC_X/$CC_LM" remote set-url origin "$CC_X/no-such-remote.git"
CC_ERR=$(cc_war "$CC_X/$CC_LM" claim "$CC_M/$CC_MB" --as cut-off 2>&1 >/dev/null); CC_S=$?
CC_LEFT=0; [[ -e "$CC_X/$CC_LM/.git/openwarrant/claims/$CC_M--$CC_MB.lock" ]] && CC_LEFT=1
git -C "$CC_X/$CC_LM" remote set-url origin "$CC_X/bare.git"
if [[ $CC_S -eq 2 && $CC_LEFT -eq 0 ]] && grep -q 'ticket.claim-remote-unreachable' <<<"$CC_ERR" && grep -q 'nothing was claimed' <<<"$CC_ERR"; then
    cc_ok "an unreachable remote is refused" "ticket.claim-remote-unreachable; no lock left on this machine"
else
    cc_fail "an unreachable remote is refused" "exit $CC_S, lock left $CC_LEFT: $CC_ERR"
fi
