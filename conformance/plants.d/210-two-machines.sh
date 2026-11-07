# shellcheck shell=bash
# Two machines, one git remote, nothing else to run (docs/TICKETS.md, "Two
# machines"). Two clones of one bare `file://` remote stand for two machines,
# set up by the how-to's own commands. Each claim is paired with its refusal.
#
# Accepted: `war init --claims-remote origin` writes [claims] remote; a claim
# taken on B shows on A, holder, workspace and lease, marked elsewhere; `war
# start` on A takes only what nobody holds; a lease that ran out on B is taken
# on A by a plain `war start`; a change pushed from A reaches the claim on B it
# concerns, once, on B's next call.
# Refused: a remote name that is not one writes nothing; with no remote nothing
# is read from the network; A cannot take B's live claim (work.claimed-by-other,
# by name); a remote that cannot be read is UNKNOWN (work.claims-remote-unknown,
# `Ready: ?`), never free; a second call hears nothing again; the claim the
# change does not concern hears nothing.

echo "== two machines: claims and notices through one git remote =="
TM_WAR="$REPO_ROOT/${WAR#./}"
tm_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tm_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
TM_T=$(mktemp -d)
TM_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1)
tm_g() { "${TM_ENV[@]}" git "$@" >/dev/null 2>&1; }
# tm_as <actor> <clone> <war args...>: stdout and stderr to $TM_T/out, $TM_T/err.
tm_as() {
    local actor="$1" at="$2"; shift 2
    (cd "$at" && "${TM_ENV[@]}" OPENWARRANT_ACTOR="$actor" timeout 120 "$TM_WAR" "$@" \
        >"$TM_T/out" 2>"$TM_T/err" </dev/null)
}
tm_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$TM_T/out"; }
tm_heard() { grep -c '^notice: ' "$TM_T/err"; }
TM_S="$TM_T/seed" TM_A="$TM_T/a" TM_B="$TM_T/b"

mkdir -p "$TM_S/docs"
tm_g -C "$TM_S" init -q -b main
tm_as owner "$TM_S" init
printf '# Plan\n\nfirst\n' >"$TM_S/docs/plan.md"
tm_as owner "$TM_S" --json create "Alice works from the plan" --body "Follow docs/plan.md for this."; TM_AL=$(tm_id)
tm_as owner "$TM_S" --json create "Bob works elsewhere" --body "Nothing cited here."; TM_BO=$(tm_id)
tm_as owner "$TM_S" --json create "Whoever is free"; TM_FR=$(tm_id)
tm_g -C "$TM_S" add -A
tm_g -C "$TM_S" commit -qm seed
tm_g clone -q --bare "$TM_S" "$TM_T/bare.git"
tm_g clone -q "file://$TM_T/bare.git" "$TM_A"
tm_g clone -q "file://$TM_T/bare.git" "$TM_B"
if [[ -z "$TM_AL" || -z "$TM_BO" || -z "$TM_FR" || ! -d "$TM_B/.git" ]]; then
    printf 'PLANT SETUP FAILED: two clones: %s\n' "$(head -c 300 "$TM_T/err")" >&2; exit 9
fi
TM_SEEN="$TM_A/.git/openwarrant/remote-claims.json"

# --- off, then one command to set it up ---------------------------------------------
tm_as carol "$TM_A" view ready; TM_RC=$?
[[ -e "$TM_SEEN" ]] && TM_OFF=read || TM_OFF=none
cp "$TM_A/openwarrant.toml" "$TM_T/before.toml"
tm_as carol "$TM_A" init --claims-remote 'or"igin'; TM_BAD=$?
if [[ $TM_RC == 0 && $TM_OFF == none && $TM_BAD != 0 ]] && cmp -s "$TM_A/openwarrant.toml" "$TM_T/before.toml"; then
    tm_ok "no remote, no network" "no [claims] remote: nothing read; a name that is no remote name writes nothing"
else
    tm_fail "no remote, no network" "ready $TM_RC, remote $TM_OFF, bad name exit $TM_BAD: $(head -c 200 "$TM_T/err")"
fi
# The how-to, as written.
tm_as carol "$TM_A" init --claims-remote origin; TM_RC=$?
(cd "$TM_A" && "${TM_ENV[@]}" git commit -qm "claims through origin" openwarrant.toml && "${TM_ENV[@]}" git push -q origin HEAD) >/dev/null 2>&1
(cd "$TM_B" && "${TM_ENV[@]}" git pull -q) >/dev/null 2>&1
if [[ $TM_RC == 0 ]] && grep -qx 'remote = "origin"' "$TM_B/openwarrant.toml"; then
    tm_ok "init --claims-remote" "[claims] remote = \"origin\", committed, pulled on the other clone"
else
    tm_fail "init --claims-remote" "exit $TM_RC: $(head -c 300 "$TM_T/err")"
fi
# Read the remote at every call, so the plant needs no sleep (default 30 s).
for c in "$TM_A" "$TM_B"; do sed -i 's/^remote = "origin"$/remote = "origin"\nremote_ttl = 0/' "$c/openwarrant.toml"; done

# --- a claim on B shows on A, and A cannot take it ------------------------------------
tm_as alice "$TM_B" start "$TM_AL"; TM_R1=$?
tm_as bob "$TM_B" start "$TM_BO"; TM_R2=$?
tm_as carol "$TM_A" view ready; TM_RC=$?; cp "$TM_T/out" "$TM_T/ready"
tm_as carol "$TM_A" view prime; cp "$TM_T/out" "$TM_T/prime"
if [[ $TM_R1 == 0 && $TM_R2 == 0 && $TM_RC == 0 ]] \
    && ! grep -q "^$TM_AL " "$TM_T/ready" && ! grep -q "^$TM_BO " "$TM_T/ready" \
    && grep -q "held elsewhere: .*$TM_AL by alice (on origin, workspace $TM_B, lease .* left)" "$TM_T/ready" \
    && grep -q "claimed by alice (.* ago, elsewhere on origin, workspace $TM_B" "$TM_T/prime"; then
    tm_ok "a claim on B shows on A" "ready leaves it out and names it held elsewhere; prime names holder, workspace and lease"
else
    tm_fail "a claim on B shows on A" "starts $TM_R1/$TM_R2, ready $TM_RC: $(head -c 300 "$TM_T/ready") | $(grep "$TM_AL" "$TM_T/prime" | head -c 200)"
fi
tm_as carol "$TM_A" start "$TM_AL"; TM_RC=$?
TM_HOLDER=$(git -C "$TM_T/bare.git" log -1 --format=%B "refs/openwarrant/claims/$TM_AL" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["actor"])' 2>/dev/null)
if [[ $TM_RC != 0 && "$TM_HOLDER" == alice ]] && grep -q 'work.claimed-by-other' "$TM_T/err" \
    && grep -q 'claimed on the remote .origin. by alice' "$TM_T/err" && [[ ! -e "$TM_A/.git/openwarrant/claims/$TM_AL.lock" ]]; then
    tm_ok "A cannot take B's claim" "refused by name, the remote still alice's, no lock kept on A"
else
    tm_fail "A cannot take B's claim" "exit $TM_RC, remote holder '$TM_HOLDER': $(head -c 300 "$TM_T/err")"
fi
tm_as carol "$TM_A" start; TM_RC=$?
TM_TOOK=$(cd "$TM_A/.git/openwarrant/claims" && ls -- *.lock 2>/dev/null | tr '\n' ' ')
tm_as carol "$TM_A" admin release "$TM_FR"
if [[ $TM_RC == 0 && "$TM_TOOK" == "$TM_FR.lock " ]]; then
    tm_ok "start takes only what is free" "war start on A took $TM_FR, never one held on B"
else
    tm_fail "start takes only what is free" "exit $TM_RC took '$TM_TOOK': $(head -c 300 "$TM_T/err")"
fi

# --- a remote that cannot be read is UNKNOWN, never free -----------------------------
tm_g -C "$TM_A" remote set-url origin "file://$TM_T/nowhere.git"
tm_as carol "$TM_A" view ready; TM_RC=$?; cp "$TM_T/err" "$TM_T/dead"
tm_as carol "$TM_A" status; cp "$TM_T/out" "$TM_T/status"
tm_g -C "$TM_A" remote set-url origin "file://$TM_T/bare.git"
tm_as carol "$TM_A" view ready; TM_LIVE=$?
if [[ $TM_RC != 0 && $TM_LIVE == 0 ]] && grep -q 'UNKNOWN (work.claims-remote-unknown)' "$TM_T/dead" \
    && grep -q '^Ready: ? tickets' "$TM_T/status" && ! grep -q 'UNKNOWN' "$TM_T/err"; then
    tm_ok "a dead remote reads UNKNOWN" "exit $TM_RC and work.claims-remote-unknown, status Ready: ?; reachable again, exit 0"
else
    tm_fail "a dead remote reads UNKNOWN" "dead exit $TM_RC, live $TM_LIVE: $(head -c 200 "$TM_T/dead") | $(grep Ready "$TM_T/status")"
fi

# --- a lease that ran out on B is taken on A ------------------------------------------
sed -i 's/^remote_ttl = 0$/remote_ttl = 0\nclaim_lease_minutes = 0.03/' "$TM_B/openwarrant.toml"
tm_as dave "$TM_B" start "$TM_FR"; TM_R1=$?
sed -i '/^claim_lease_minutes = 0.03$/d' "$TM_B/openwarrant.toml"
sleep 3
tm_as carol "$TM_A" view ready; cp "$TM_T/out" "$TM_T/ready"
tm_as carol "$TM_A" start "$TM_FR"; TM_RC=$?
TM_HOLDER=$(git -C "$TM_T/bare.git" log -1 --format=%B "refs/openwarrant/claims/$TM_FR" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["actor"])' 2>/dev/null)
if [[ $TM_R1 == 0 && $TM_RC == 0 && "$TM_HOLDER" == carol ]] \
    && grep -q "^$TM_FR .*\[lease ran out: dave held it\]" "$TM_T/ready"; then
    tm_ok "a lease run out on B is A's" "offered on A as run out, taken by a plain war start, the remote's ref carol's"
else
    tm_fail "a lease run out on B is A's" "B start $TM_R1, A start $TM_RC, holder '$TM_HOLDER': $(head -c 300 "$TM_T/err") | $(head -c 200 "$TM_T/ready")"
fi

# --- a change pushed from A reaches only the claim it concerns on B -------------------
printf '# Plan\n\nchanged on A\n' >"$TM_A/docs/plan.md"
(cd "$TM_A" && "${TM_ENV[@]}" git commit -qam "the plan changes" && "${TM_ENV[@]}" git push -q origin main) >/dev/null 2>&1
tm_as alice "$TM_B" view ready; TM_H1=$(tm_heard); cp "$TM_T/err" "$TM_T/heard"
tm_as alice "$TM_B" view ready; TM_H2=$(tm_heard)
tm_as bob "$TM_B" view ready; TM_H3=$(tm_heard)
TM_BOBS=$(grep -l "\"$TM_BO\"" "$TM_B/.git/openwarrant/notices"/*.json "$TM_B/.git/openwarrant/notices"/delivered/*.json 2>/dev/null | wc -l)
if [[ $TM_H1 == 1 && $TM_H2 == 0 && $TM_H3 == 0 && $TM_BOBS == 0 ]] \
    && grep -q "^notice: docs/plan.md changed on origin/main; run .war start $TM_AL." "$TM_T/heard"; then
    tm_ok "a notice reaches only its claim" "alice on B heard it once on her next call; again nothing; bob nothing"
else
    tm_fail "a notice reaches only its claim" "alice $TM_H1 then $TM_H2, bob $TM_H3 (files $TM_BOBS): $(head -c 300 "$TM_T/heard")"
fi

[[ -n "$TM_T" && "$TM_T" == */tmp.* ]] && /usr/bin/rm -rf "$TM_T"
