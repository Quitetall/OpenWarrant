# shellcheck shell=bash
# `war admin migrate` (R5, docs/TICKETS.md "Moving older tickets"): tickets
# kept outside docs/work/ become masters, ids kept. Each claim is paired with
# its refusal.
#
# Accepted: `--dry-run` prints every move and writes nothing; the real run
# performs exactly that plan with `git mv` (renames in the index); a
# single-file ticket's bytes are unchanged and a directory ticket is written
# as one document; journals come along; the views list the same work, in the
# same states, before and after; `war check` passes; the migrated `t-` id is
# worked where it now is.
# Refused: a ticket with a recorded act and one with a live claim are named
# and left in place, byte for byte; a second run moves nothing.

echo "== one layout: war admin migrate =="
MW_WAR="$REPO_ROOT/${WAR#./}"
mw_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
mw_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
MW_T=$(mktemp -d)
MW_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1)
mw_g() { "${MW_ENV[@]}" git "$@" >/dev/null 2>&1; }
mw_as() {
    local actor="$1"; shift
    (cd "$MW_R" && "${MW_ENV[@]}" OPENWARRANT_ACTOR="$actor" timeout 120 "$MW_WAR" "$@" \
        >"$MW_T/out" 2>"$MW_T/err" </dev/null)
}
mw_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$MW_T/out"; }
# The work as the views list it: id, state, progress and title of each ticket.
mw_listed() {
    mw_as owner --json view tickets
    python3 -c 'import json,sys
for t in sorted(json.load(open(sys.argv[1]))["result"]["tickets"], key=lambda t: t["id"]):
    print(t["id"], t["state"], t["done"], t["total"], t["title"])' "$MW_T/out"
}
# mw_single <alias> <title>: a single-file ticket as `war create` wrote one before docs/work/.
mw_single() {
    cat >"$MW_R/docs/tickets/$1.md" <<DOC
---
format: openwarrant/1.0
profile: ticket
alias: $1
uuid: 01a1d2f8-0000-7000-8000-00000000${1#t-}
priority: 2
created_at: "2026-11-01T00:00:00Z"
created_by: "owner"
summary: "$2"
---
# $2

## Checklist

- [ ] first step (i-${1#t-}1)
- [ ] second step (i-${1#t-}2)
DOC
}
MW_R="$MW_T/r"

mkdir -p "$MW_R/docs/tickets"
mw_g -C "$MW_R" init -q -b main
mw_as owner init
mw_single t-aa01 "A single file ticket"
mw_single t-aa02 "One that is claimed"
mw_single t-aa03 "One under a recorded act"
mw_as owner --json create "A directory ticket" --legacy --item "alpha"; MW_DIR=$(mw_id)
# Worked a little, so a journal and a tick come along.
mw_as owner start t-aa01; mw_as owner done t-aa01/i-aa011 --note "did the first"; mw_as owner admin release t-aa01
mkdir -p "$MW_R/docs/tickets/t-aa03"
printf 'schema = "oh.war/authorization/v1"\n' >"$MW_R/docs/tickets/t-aa03/authorization.toml"
mw_g -C "$MW_R" add -A
mw_g -C "$MW_R" commit -qm seed
mw_as bob start t-aa02
if [[ -z "$MW_DIR" || ! -f "$MW_R/docs/tickets/$MW_DIR/manifest.toml" || ! -f "$MW_R/docs/tickets/t-aa01/journal.jsonl" ]]; then
    printf 'PLANT SETUP FAILED: four older tickets: %s\n' "$(head -c 300 "$MW_T/err")" >&2; exit 9
fi
mw_listed >"$MW_T/before"
cp "$MW_R/docs/tickets/t-aa01.md" "$MW_T/single.md"
cp "$MW_R/docs/tickets/t-aa02.md" "$MW_T/claimed.md"
mw_status() { (cd "$MW_R" && "${MW_ENV[@]}" git status --short | grep -v '\.openwarrant/'); }

# --- --dry-run prints the plan and writes nothing --------------------------------------
mw_status >"$MW_T/status0"
mw_as owner admin migrate --dry-run; MW_RC=$?; cp "$MW_T/out" "$MW_T/dry"
mw_status >"$MW_T/status1"
if [[ $MW_RC == 0 ]] && cmp -s "$MW_T/status0" "$MW_T/status1" && [[ ! -e "$MW_R/docs/work" ]] \
    && grep -q "would move docs/tickets/t-aa01.md -> docs/work/t-aa01/t-aa01-a-single-file-ticket.md" "$MW_T/dry" \
    && grep -q "would write docs/work/$MW_DIR/$MW_DIR-a-directory-ticket.md" "$MW_T/dry"; then
    mw_ok "--dry-run writes nothing" "every move printed; the tree is as it was"
else
    mw_fail "--dry-run writes nothing" "exit $MW_RC: $(head -c 400 "$MW_T/dry")"
fi

# --- acted-on and claimed tickets are named and left in place ------------------------
if grep -q "^t-aa02  left in place (migrate.claimed): bob holds t-aa02" "$MW_T/dry" \
    && grep -q "^t-aa03  left in place (migrate.recorded-act): docs/tickets/t-aa03/authorization.toml" "$MW_T/dry" \
    && ! grep -q "would move docs/tickets/t-aa0[23]" "$MW_T/dry"; then
    mw_ok "what is held or acted on stays" "the claimed ticket and the one under a recorded act are named, with why, and not in the plan"
else
    mw_fail "what is held or acted on stays" "$(grep -a 'left in place\|t-aa0[23]' "$MW_T/dry" | head -c 400)"
fi

# --- the real run does exactly the plan, by git mv -------------------------------------
mw_as owner admin migrate; MW_RC=$?; cp "$MW_T/out" "$MW_T/real"
sed -e 's/would move/moved/; s/would write/wrote/; s/would remove/removed/' "$MW_T/dry" | grep -v 'ticket(s)' >"$MW_T/dry.as-done"
grep -v 'ticket(s)' "$MW_T/real" >"$MW_T/real.plan"
MW_RENAMED=$(mw_status | grep -c '^R  docs/tickets/t-aa01.md -> docs/work/t-aa01/t-aa01-a-single-file-ticket.md')
if [[ $MW_RC == 0 && $MW_RENAMED == 1 ]] && cmp -s "$MW_T/dry.as-done" "$MW_T/real.plan" \
    && cmp -s "$MW_T/single.md" "$MW_R/docs/work/t-aa01/t-aa01-a-single-file-ticket.md" \
    && [[ -f "$MW_R/docs/work/t-aa01/journal.jsonl" && ! -e "$MW_R/docs/work/$MW_DIR/manifest.toml" ]] \
    && grep -q '^alias: '"$MW_DIR"'$' "$MW_R/docs/work/$MW_DIR/$MW_DIR-a-directory-ticket.md" \
    && cmp -s "$MW_T/claimed.md" "$MW_R/docs/tickets/t-aa02.md" && [[ -f "$MW_R/docs/tickets/t-aa03/authorization.toml" ]]; then
    mw_ok "the run is the plan, by git mv" "same lines as the dry run; a rename in the index; bytes and journal carried; the two kept tickets untouched"
else
    mw_fail "the run is the plan, by git mv" "exit $MW_RC renamed $MW_RENAMED: $(diff "$MW_T/dry.as-done" "$MW_T/real.plan" | head -c 300) | $(mw_status | head -c 300)"
fi

# --- the same work before and after; check passes; the id still works ------------------
mw_listed >"$MW_T/after"
mw_as owner check; MW_CHECK=$?
mw_as owner start t-aa01; MW_S=$?
mw_as owner done t-aa01/i-aa012; MW_D=$?
if cmp -s "$MW_T/before" "$MW_T/after" && [[ $(wc -l <"$MW_T/before") == 4 && $MW_CHECK == 0 && $MW_S == 0 && $MW_D == 0 ]] \
    && grep -q '^- \[x\] second step' "$MW_R/docs/work/t-aa01/t-aa01-a-single-file-ticket.md"; then
    mw_ok "the same work, the same ids" "four tickets listed alike before and after; war check passes; t-aa01 is started and ticked where it now is"
else
    mw_fail "the same work, the same ids" "check $MW_CHECK start $MW_S done $MW_D: $(diff "$MW_T/before" "$MW_T/after" | head -c 300) $(head -c 200 "$MW_T/err")"
fi

# --- a second run moves nothing ----------------------------------------------------------
mw_status >"$MW_T/status2"
mw_as owner admin migrate; MW_RC=$?
mw_status >"$MW_T/status3"
if [[ $MW_RC == 0 ]] && cmp -s "$MW_T/status2" "$MW_T/status3" && grep -q '^0 ticket(s) moved, 2 left in place' "$MW_T/out"; then
    mw_ok "a second run changes nothing" "nothing left to move; the two kept tickets are named again"
else
    mw_fail "a second run changes nothing" "exit $MW_RC: $(head -c 300 "$MW_T/out")"
fi

[[ -n "$MW_T" && "$MW_T" == */tmp.* ]] && /usr/bin/rm -rf "$MW_T"
