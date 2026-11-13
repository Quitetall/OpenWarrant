# shellcheck shell=bash
# Shared work across clones (R2, docs/TICKETS.md "Shared work"): with
# `[claims] remote`, a new ticket is also a ref on that remote. Two clones of
# one bare `file://` remote stand for two machines. Each claim is paired with
# its refusal.
#
# Accepted: a ticket created in clone 1 is a ref whose commit carries its
# document; clone 2 lists it with its title, marked as on the remote, with
# nothing fetched into a branch; `war start` in clone 2 writes its document
# there; done in clone 2, the ref is gone.
# Refused: with no remote configured nothing is read and nothing is listed;
# clone 2 creating the same title is `ticket.duplicate`, naming clone 1's;
# what was read stands for `remote_ttl` and no longer; a remote that cannot be
# read is UNKNOWN (non-zero, `Ready: ?`), never an empty list.

echo "== shared work: tickets through the claims remote =="
SR_WAR="$REPO_ROOT/${WAR#./}"
sr_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sr_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
SR_T=$(mktemp -d)
SR_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1)
sr_g() { "${SR_ENV[@]}" git "$@" >/dev/null 2>&1; }
sr_as() {
    local actor="$1" at="$2"; shift 2
    (cd "$at" && "${SR_ENV[@]}" OPENWARRANT_ACTOR="$actor" timeout 120 "$SR_WAR" "$@" \
        >"$SR_T/out" 2>"$SR_T/err" </dev/null)
}
sr_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$SR_T/out"; }
SR_S="$SR_T/seed" SR_1="$SR_T/one" SR_2="$SR_T/two" SR_BARE="$SR_T/bare.git"

mkdir -p "$SR_S"
sr_g -C "$SR_S" init -q -b main
sr_as owner "$SR_S" init
sr_g -C "$SR_S" add -A
sr_g -C "$SR_S" commit -qm seed
sr_g clone -q --bare "$SR_S" "$SR_BARE"
sr_g clone -q "file://$SR_BARE" "$SR_1"
sr_g clone -q "file://$SR_BARE" "$SR_2"
sr_as one "$SR_1" init --claims-remote origin; SR_SET=$?
if [[ $SR_SET != 0 || ! -d "$SR_2/.git" ]]; then
    printf 'PLANT SETUP FAILED: two clones: %s\n' "$(head -c 300 "$SR_T/err")" >&2; exit 9
fi
SR_SEEN="$SR_2/.git/openwarrant/remote-tickets.json"

# --- created in clone 1: a ref carrying the document ---------------------------------
sr_as one "$SR_1" --json create "Ship the audit log" --item "write it"; SR_RC=$?; SR_ID=$(sr_id)
SR_REF="refs/openwarrant/tickets/$SR_ID"
SR_TITLE=$(git -C "$SR_BARE" log -1 --format=%B "$SR_REF" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["title"])' 2>/dev/null)
SR_BLOB=$(git -C "$SR_BARE" show "$SR_REF:$SR_ID.md" 2>/dev/null | grep -c '^# Ship the audit log')
if [[ $SR_RC == 0 && -n "$SR_ID" && "$SR_TITLE" == "Ship the audit log" && $SR_BLOB == 1 ]]; then
    sr_ok "a new ticket is a ref" "refs/openwarrant/tickets/<id>: its record in the commit message, its document a blob"
else
    sr_fail "a new ticket is a ref" "exit $SR_RC id '$SR_ID' title '$SR_TITLE' blob $SR_BLOB: $(head -c 300 "$SR_T/err")"
fi

# --- no remote configured: nothing read, nothing listed ------------------------------
sr_as two "$SR_2" view tickets; SR_RC=$?
SR_LISTED=$(grep -c "$SR_ID" "$SR_T/out")
[[ -e "$SR_SEEN" ]] && SR_OFF=read || SR_OFF=none
if [[ $SR_RC == 0 && $SR_LISTED == 0 && $SR_OFF == none ]]; then
    sr_ok "no remote, no network" "clone 2 without [claims] remote: nothing asked of the remote, nothing listed"
else
    sr_fail "no remote, no network" "exit $SR_RC listed $SR_LISTED seen-file $SR_OFF"
fi

# --- with the remote: shown in clone 2, by title ---------------------------------------
sr_as two "$SR_2" init --claims-remote origin
printf 'remote_ttl = 3\n' >>"$SR_2/openwarrant.toml"
sr_as two "$SR_2" view tickets; SR_RC=$?; cp "$SR_T/out" "$SR_T/listed"
sr_as two "$SR_2" status; cp "$SR_T/out" "$SR_T/status"
if [[ $SR_RC == 0 && ! -e "$SR_2/docs/tickets/$SR_ID.md" ]] \
    && grep -q "^$SR_ID .*Ship the audit log .*\[elsewhere: remote origin" "$SR_T/listed" \
    && grep -q '^Tickets: 1 open' "$SR_T/status"; then
    sr_ok "created in 1, shown in 2" "listed and counted in clone 2 by title, marked as on the remote; no file fetched into the tree"
else
    sr_fail "created in 1, shown in 2" "exit $SR_RC: $(head -c 300 "$SR_T/listed") | $(head -c 200 "$SR_T/err")"
fi

# --- the same title in clone 2 is refused, naming clone 1's ---------------------------
sr_as two "$SR_2" create "Ship the audit log"; SR_RC=$?
if [[ $SR_RC != 0 ]] && grep -q "ticket.duplicate.*$SR_ID (\"Ship the audit log\").*remote origin" "$SR_T/err" \
    && [[ $(find "$SR_2/docs/tickets" -name 't-*.md' 2>/dev/null | wc -l) == 0 ]]; then
    sr_ok "a duplicate across clones" "refused in clone 2 naming clone 1's ticket on the remote; nothing written"
else
    sr_fail "a duplicate across clones" "exit $SR_RC: $(head -c 300 "$SR_T/err")"
fi

# --- read once per remote_ttl; a dead remote is UNKNOWN, never nothing ----------------
mv "$SR_BARE" "$SR_T/gone.git"
sr_as two "$SR_2" view tickets; SR_CACHED=$?; SR_STILL=$(grep -c "^$SR_ID .*Ship the audit log" "$SR_T/out")
sleep 4
sr_as two "$SR_2" view ready; SR_DEAD=$?; cp "$SR_T/err" "$SR_T/dead"
sr_as two "$SR_2" status; cp "$SR_T/out" "$SR_T/status"
mv "$SR_T/gone.git" "$SR_BARE"
if [[ $SR_CACHED == 0 && $SR_STILL == 1 && $SR_DEAD != 0 ]] \
    && grep -q 'UNKNOWN.*tickets announced on the remote `origin` are UNKNOWN' "$SR_T/dead" \
    && grep -q '^Ready: ?' "$SR_T/status"; then
    sr_ok "a dead remote is UNKNOWN" "inside remote_ttl what was read stands; read again, the remote is UNKNOWN and ready work is ?"
else
    sr_fail "a dead remote is UNKNOWN" "cached $SR_CACHED still $SR_STILL dead $SR_DEAD: $(head -c 300 "$SR_T/dead") | $(head -c 120 "$SR_T/status")"
fi

# --- worked and closed in clone 2: the document comes over, then the ref goes --------
sleep 4
sr_as two "$SR_2" start "$SR_ID"; SR_RC=$?
SR_SAME=no; cmp -s "$SR_1/docs/tickets/$SR_ID.md" "$SR_2/docs/tickets/$SR_ID.md" && SR_SAME=yes
SR_HELD=$(git -C "$SR_BARE" for-each-ref "$SR_REF" | wc -l)
sr_as two "$SR_2" done "$SR_ID"; SR_DONE=$?
SR_LEFT=$(git -C "$SR_BARE" for-each-ref "$SR_REF" | wc -l)
if [[ $SR_RC == 0 && $SR_SAME == yes && $SR_HELD == 1 && $SR_DONE == 0 && $SR_LEFT == 0 ]]; then
    sr_ok "worked in 2, then retired" "war start wrote clone 1's bytes into clone 2; done there, the ref is gone"
else
    sr_fail "worked in 2, then retired" "start $SR_RC same $SR_SAME ref before $SR_HELD after $SR_LEFT done $SR_DONE: $(head -c 300 "$SR_T/err")"
fi

[[ -n "$SR_T" && "$SR_T" == */tmp.* ]] && /usr/bin/rm -rf "$SR_T"
