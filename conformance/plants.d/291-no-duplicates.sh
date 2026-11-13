# shellcheck shell=bash
# No two agents make the same ticket (R2, docs/TICKETS.md "Shared work").
# Two worktrees of one clone. Each claim is paired with its refusal.
#
# Accepted: distinct work that shares most of its words is created ("Add CSV
# export" beside "Add JSON export"); `--force` creates a likely duplicate; a
# title whose first ticket is done is created again.
# Refused: a reworded title of an open ticket, from another worktree, is
# `ticket.duplicate`, naming the first by id, title and worktree, and nothing
# is written; of six creates of one title started at once in two worktrees,
# exactly one makes a ticket and five are refused naming it.

echo "== shared work: no duplicate tickets =="
ND_WAR="$REPO_ROOT/${WAR#./}"
nd_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
nd_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ND_T=$(mktemp -d)
ND_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1)
nd_g() { "${ND_ENV[@]}" git "$@" >/dev/null 2>&1; }
nd_as() {
    local actor="$1" at="$2"; shift 2
    (cd "$at" && "${ND_ENV[@]}" OPENWARRANT_ACTOR="$actor" timeout 120 "$ND_WAR" "$@" \
        >"$ND_T/out" 2>"$ND_T/err" </dev/null)
}
nd_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$ND_T/out"; }
nd_count() { find "$ND_A/docs/work" "$ND_B/docs/work" -name '*.md' 2>/dev/null | wc -l; }
ND_A="$ND_T/a" ND_B="$ND_T/b"

mkdir -p "$ND_A"
nd_g -C "$ND_A" init -q -b main
nd_as owner "$ND_A" init
nd_g -C "$ND_A" add -A
nd_g -C "$ND_A" commit -qm seed
nd_g -C "$ND_A" worktree add -q "$ND_B" -b feature
nd_as alice "$ND_A" --json create "Add CSV export"; ND_FIRST=$(nd_id)
if [[ -z "$ND_FIRST" || ! -f "$ND_B/openwarrant.toml" ]]; then
    printf 'PLANT SETUP FAILED: two worktrees and a ticket: %s\n' "$(head -c 300 "$ND_T/err")" >&2; exit 9
fi

# --- a reworded duplicate is refused, naming the first -----------------------------
ND_N=$(nd_count)
nd_as bob "$ND_B" create "Add export to CSV"; ND_RC=$?
if [[ $ND_RC != 0 && $(nd_count) == "$ND_N" ]] \
    && grep -q "ticket.duplicate.*$ND_FIRST (\"Add CSV export\").*worktree $ND_A.*war start $ND_FIRST" "$ND_T/err"; then
    nd_ok "a reworded duplicate is refused" "from another worktree: ticket.duplicate names the id, title and worktree; nothing written"
else
    nd_fail "a reworded duplicate is refused" "exit $ND_RC, $(nd_count) tickets (was $ND_N): $(head -c 300 "$ND_T/err")"
fi

# --- distinct work with similar words is created ------------------------------------
nd_as bob "$ND_B" --json create "Add JSON export"; ND_RC=$?; ND_JSON=$(nd_id)
if [[ $ND_RC == 0 && -n "$ND_JSON" && -f "$(tk "$ND_B/docs/tickets/$ND_JSON.md")" ]]; then
    nd_ok "distinct work is created" "\"Add JSON export\" beside \"Add CSV export\": one word differs on each side, and it is the work"
else
    nd_fail "distinct work is created" "exit $ND_RC: $(head -c 300 "$ND_T/err")"
fi

# --- --force creates ------------------------------------------------------------------
nd_as bob "$ND_B" --json create "Add export to CSV" --force; ND_RC=$?; ND_FORCED=$(nd_id)
if [[ $ND_RC == 0 && -n "$ND_FORCED" && "$ND_FORCED" != "$ND_FIRST" && -f "$(tk "$ND_B/docs/tickets/$ND_FORCED.md")" ]]; then
    nd_ok "--force creates" "the same title that was refused is created when asked for outright"
else
    nd_fail "--force creates" "exit $ND_RC: $(head -c 300 "$ND_T/err")"
fi

# --- six creates of one title at once leave one ticket -------------------------------
for n in 1 2 3 4 5 6; do
    at="$ND_A"; (( n % 2 == 0 )) && at="$ND_B"
    (cd "$at" && "${ND_ENV[@]}" OPENWARRANT_ACTOR="racer$n" timeout 120 "$ND_WAR" create "Race for one title" \
        >"$ND_T/race$n.out" 2>"$ND_T/race$n.err" </dev/null; echo $? >"$ND_T/race$n.rc") &
done
wait
ND_WON=0 ND_NAMED=0
for n in 1 2 3 4 5 6; do
    [[ $(cat "$ND_T/race$n.rc") == 0 ]] && ND_WON=$((ND_WON + 1))
    grep -q 'ticket.duplicate.*"Race for one title"' "$ND_T/race$n.err" && ND_NAMED=$((ND_NAMED + 1))
done
ND_MADE=$(grep -l '^# Race for one title' "$ND_A"/docs/tickets/t-*.md "$ND_A"/docs/work/*/*.md "$ND_B"/docs/tickets/t-*.md "$ND_B"/docs/work/*/*.md 2>/dev/null | wc -l)
if [[ $ND_WON == 1 && $ND_NAMED == 5 && $ND_MADE == 1 ]]; then
    nd_ok "a raced title leaves one ticket" "six at once in two worktrees: one created, five refused naming it"
else
    nd_fail "a raced title leaves one ticket" "$ND_WON created, $ND_NAMED refused by name, $ND_MADE documents: $(head -c 200 "$ND_T/race1.err")"
fi

# --- a closed ticket never blocks ------------------------------------------------------
nd_as alice "$ND_A" --json create "Tidy the changelog"; ND_OLD=$(nd_id)
nd_as bob "$ND_B" create "Tidy the changelog"; ND_OPEN=$?
nd_as alice "$ND_A" start "$ND_OLD"; nd_as alice "$ND_A" done "$ND_OLD"; ND_DONE=$?
nd_as alice "$ND_A" --json create "Tidy the changelog"; ND_RC=$?; ND_NEW=$(nd_id)
if [[ $ND_OPEN != 0 && $ND_DONE == 0 && $ND_RC == 0 && -n "$ND_NEW" && "$ND_NEW" != "$ND_OLD" ]]; then
    nd_ok "a closed ticket never blocks" "refused while the first is open; created once it is done"
else
    nd_fail "a closed ticket never blocks" "while open $ND_OPEN, done $ND_DONE, after $ND_RC: $(head -c 300 "$ND_T/err")"
fi

[[ -n "$ND_T" && "$ND_T" == */tmp.* ]] && /usr/bin/rm -rf "$ND_T"
