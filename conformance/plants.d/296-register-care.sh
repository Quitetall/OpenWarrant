# shellcheck shell=bash
# The clone's register stays true to the files (R2/R5, docs/TICKETS.md
# "Shared work"). Two worktrees of one clone. Each claim is paired with its
# refusal.
#
# Accepted: a ticket deleted by hand in the worktree that created it, on the
# branch it was created on, loses its register entry on the next create, so
# its title is free again and no worktree is offered it; a master another
# worktree holds is read by `war status <id>` and `war check <id>` as the
# ticket it is, before `war start` brings it here.
# Refused: switching that worktree to another branch (where the file is not)
# retires nothing: the entry stays and the title is still refused by name.

echo "== shared work: the register follows the files =="
RC_WAR="$REPO_ROOT/${WAR#./}"
rc_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
RC_T=$(mktemp -d)
RC_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1)
rc_g() { "${RC_ENV[@]}" git "$@" >/dev/null 2>&1; }
rc_as() {
    local actor="$1" at="$2"; shift 2
    (cd "$at" && "${RC_ENV[@]}" OPENWARRANT_ACTOR="$actor" timeout 120 "$RC_WAR" "$@" \
        >"$RC_T/out" 2>"$RC_T/err" </dev/null)
}
rc_made() { python3 -c 'import json,sys; r=json.load(open(sys.argv[1]))["result"]; print(r["id"], r["source"])' "$RC_T/out"; }
RC_A="$RC_T/a" RC_B="$RC_T/b"

mkdir -p "$RC_A"
rc_g -C "$RC_A" init -q -b main
rc_as owner "$RC_A" init
rc_g -C "$RC_A" add -A
rc_g -C "$RC_A" commit -qm seed
rc_g -C "$RC_A" worktree add -q "$RC_B" -b feature
rc_g -C "$RC_A" branch other
RC_REG="$RC_A/.git/openwarrant/tickets"
rc_as alice "$RC_A" --json create "Tune the retry backoff" --item "measure"; read -r RC_ID RC_SRC < <(rc_made)
if [[ -z "$RC_ID" || ! -f "$RC_A/$RC_SRC" || ! -f "$RC_REG/$RC_ID" ]]; then
    printf 'PLANT SETUP FAILED: a registered ticket: %s\n' "$(head -c 300 "$RC_T/err")" >&2; exit 9
fi

# --- a branch switch retires nothing ---------------------------------------------------
rc_g -C "$RC_A" stash -u
rc_g -C "$RC_A" checkout -q other
rc_as bob "$RC_B" create "Tune the retry backoff"; RC_RC=$?; cp "$RC_T/err" "$RC_T/err.dup"
RC_KEPT=no; [[ -f "$RC_REG/$RC_ID" ]] && RC_KEPT=yes
rc_as alice "$RC_A" create "Something on the other branch"; RC_OTHER=$?
[[ -f "$RC_REG/$RC_ID" ]] || RC_KEPT=no
rc_g -C "$RC_A" checkout -q main
rc_g -C "$RC_A" stash pop
if [[ $RC_RC != 0 && $RC_OTHER == 0 && $RC_KEPT == yes && -f "$RC_A/$RC_SRC" ]] && grep -q "ticket.duplicate.*$RC_ID" "$RC_T/err.dup"; then
    rc_ok "a branch switch retires nothing" "the worktree on another branch, the file not there: the entry stays and the title is still refused"
else
    rc_fail "a branch switch retires nothing" "duplicate create $RC_RC, create on the other branch $RC_OTHER, entry kept $RC_KEPT"
fi

# --- deleted by hand on its own branch: the entry goes, the title is free ---------------
rm -rf "${RC_A:?}/$RC_SRC" "${RC_A:?}/docs/tickets/$RC_ID" "${RC_A:?}/docs/work/$RC_ID"
rc_as bob "$RC_B" view tickets; RC_LISTED=$(grep -c "$RC_ID" "$RC_T/out")
rc_as alice "$RC_A" --json create "Tune the retry backoff"; RC_RC=$?; read -r RC_NEW _ < <(rc_made)
if [[ $RC_LISTED -le 1 && $RC_RC == 0 && -n "$RC_NEW" && "$RC_NEW" != "$RC_ID" && ! -e "$RC_REG/$RC_ID" ]]; then
    rc_ok "a deleted ticket is retired" "gone from the working tree and HEAD on its own branch: its entry is dropped and its title is free"
else
    rc_fail "a deleted ticket is retired" "listed $RC_LISTED create $RC_RC new '$RC_NEW': $(head -c 300 "$RC_T/err")"
fi

# --- a master held elsewhere is read as a ticket before war start ----------------------
plant_layout work "a w- id nobody has locally routes as a ticket; a t- id always did"
rc_as alice "$RC_A" --json create "Rotate the audit keys" --item "one"; read -r RC_W _ < <(rc_made)
rc_as bob "$RC_B" status "$RC_W"; RC_S=$?; RC_SHOWN=$(grep -c "^# $RC_W — Rotate the audit keys" "$RC_T/out")
rc_as bob "$RC_B" --json check "$RC_W"; RC_C=$?; RC_UNKNOWN=$(grep -c 'ticket.unknown\|no Warrant' "$RC_T/out")
rc_as bob "$RC_B" status w-0000zz; RC_NONE=$?
if [[ "$RC_W" == w-* && $RC_S == 0 && $RC_SHOWN == 1 && $RC_C == 0 && $RC_UNKNOWN == 0 && $RC_NONE != 0 && ! -e "$RC_B/docs/work/$RC_W" ]]; then
    rc_ok "an elsewhere master routes" "status and check in B read A's uncommitted w- ticket; an id nobody announced is refused"
else
    rc_fail "an elsewhere master routes" "id '$RC_W' status $RC_S shown $RC_SHOWN check $RC_C unknown $RC_UNKNOWN none $RC_NONE: $(head -c 300 "$RC_T/err")"
fi

[[ -n "$RC_T" && "$RC_T" == */tmp.* ]] && /usr/bin/rm -rf "$RC_T"
