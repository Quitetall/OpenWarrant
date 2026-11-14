# shellcheck shell=bash
# An id two clones mint at once is one clone's (R2, docs/TICKETS.md "Shared
# work"): the first push of `refs/openwarrant/tickets/<id>` expects no ref, so
# the remote settles it. Two clones of one bare `file://` remote. The claim is
# paired with its refusal.
#
# Refused: clone 2's create, whose push finds the ref another clone put there
# a moment before (a `git` on PATH plants it between the mint and the push),
# is `ticket.id-taken`, naming the holder's ticket; no document, no state
# directory and no register entry are left, and the remote's ref is still the
# other clone's.
# Accepted: the same create with nothing in the way makes the ticket and the
# ref is its own.

plant_layout work "plants a docs/work/<id>/ directory another clone settled first"
echo "== shared work: an id lost at the remote =="
IT_WAR="$REPO_ROOT/${WAR#./}"
it_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
it_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
IT_T=$(mktemp -d)
IT_GIT=$(command -v git)
IT_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1)
it_g() { "${IT_ENV[@]}" "$IT_GIT" "$@" >/dev/null 2>&1; }
# it_as <actor> <clone> <war args...>; IT_PATH, when set, goes first on PATH.
it_as() {
    local actor="$1" at="$2"; shift 2
    (cd "$at" && "${IT_ENV[@]}" OPENWARRANT_ACTOR="$actor" "PATH=${IT_PATH:+$IT_PATH:}$PATH" timeout 120 "$IT_WAR" "$@" \
        >"$IT_T/out" 2>"$IT_T/err" </dev/null)
}
IT_S="$IT_T/seed" IT_2="$IT_T/two" IT_BARE="$IT_T/bare.git"

mkdir -p "$IT_S" "$IT_T/bin"
it_g -C "$IT_S" init -q -b main
it_as owner "$IT_S" init --claims-remote origin
it_g -C "$IT_S" add -A
it_g -C "$IT_S" commit -qm seed
it_g clone -q --bare "$IT_S" "$IT_BARE"
it_g clone -q "file://$IT_BARE" "$IT_2"
# What another clone announced under the id: its record on an empty tree.
IT_TREE=$("$IT_GIT" -C "$IT_BARE" hash-object -t tree -w --stdin </dev/null)
IT_OTHER=$("${IT_ENV[@]}" "$IT_GIT" -C "$IT_BARE" commit-tree "$IT_TREE" -m \
    '{"schema":"oh.war/ticket-entry/v1","id":"ID","uuid":"01a1d2f8-0000-7000-8000-0000000000ff","title":"Theirs, a moment sooner","level":"Ticket","state":"open","created_by":"clone-one","root":"/elsewhere","path":"docs/work/ID/ID.md","blobs":{}}')
# A git that, asked to push a ticket ref, lets the other clone's land first.
cat >"$IT_T/bin/git" <<WRAP
#!/usr/bin/env bash
for a in "\$@"; do
    case "\$a" in
        *:refs/openwarrant/tickets/*) [[ " \$* " == *" push "* ]] && "$IT_GIT" -C "$IT_BARE" update-ref "\${a#*:}" "$IT_OTHER" ;;
    esac
done
exec "$IT_GIT" "\$@"
WRAP
chmod +x "$IT_T/bin/git"
if [[ -z "$IT_OTHER" || ! -d "$IT_2/.git" ]] || ! grep -q '^remote = "origin"' "$IT_2/openwarrant.toml"; then
    printf 'PLANT SETUP FAILED: a clone with a claims remote: %s\n' "$(head -c 300 "$IT_T/err")" >&2; exit 9
fi

# --- the push loses: refused by name, nothing left --------------------------------------
IT_PATH="$IT_T/bin" it_as two "$IT_2" create "Mine, a moment later" --item "do it"; IT_RC=$?
IT_REF=$("$IT_GIT" -C "$IT_BARE" for-each-ref --format='%(refname) %(objectname)' 'refs/openwarrant/tickets/*')
IT_LEFT=$(find "$IT_2/docs/work" "$IT_2/.git/openwarrant/tickets" -mindepth 1 ! -name '.*' 2>/dev/null | wc -l)
if [[ $IT_RC != 0 && $IT_LEFT == 0 && "$IT_REF" == refs/openwarrant/tickets/w-*" $IT_OTHER" ]] \
    && grep -q 'ticket.id-taken.*is already "Theirs, a moment sooner" by clone-one.*nothing was created' "$IT_T/err"; then
    it_ok "an id lost at the remote" "ticket.id-taken names the other clone's ticket; no document, state or register entry left; the ref is theirs"
else
    it_fail "an id lost at the remote" "exit $IT_RC left $IT_LEFT ref '$IT_REF': $(head -c 400 "$IT_T/err")"
fi

# --- with nothing in the way the create lands, and the ref is its own -----------------
it_as two "$IT_2" --json create "Mine, a moment later" --item "do it"; IT_RC=$?
IT_ID=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$IT_T/out" 2>/dev/null)
IT_MINE=$("$IT_GIT" -C "$IT_BARE" log -1 --format=%B "refs/openwarrant/tickets/$IT_ID" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["created_by"])' 2>/dev/null)
if [[ $IT_RC == 0 && -n "$IT_ID" && "$IT_MINE" == two && -d "$IT_2/docs/work/$IT_ID" ]]; then
    it_ok "an id won at the remote" "the same create, unopposed: the ticket is made and the ref carries its record"
else
    it_fail "an id won at the remote" "exit $IT_RC id '$IT_ID' by '$IT_MINE': $(head -c 300 "$IT_T/err")"
fi

[[ -n "$IT_T" && "$IT_T" == */tmp.* ]] && /usr/bin/rm -rf "$IT_T"
