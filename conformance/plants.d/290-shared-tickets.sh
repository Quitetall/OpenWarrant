# shellcheck shell=bash
# Shared work (R2, docs/TICKETS.md "Shared work"): a ticket made in one
# worktree is every worktree's at once. Two worktrees of one clone. Each claim
# is paired with its refusal.
#
# Accepted: a ticket created in A is counted by B's `war status` and listed
# with its title, marked with where it lives, before anything is committed;
# `war start` in B writes its document there at the same path; A adds an item
# and B ticks one, and the two branches merge with no conflict, keeping both.
# Refused: a ticket closed in A is no longer offered in B; a ticket B has no
# announcement for is `ticket.unknown` there; two tickets under one id (two
# UUIDs) are `ticket.id-collision`, naming both, the local one still listed;
# an id the register holds is never minted twice.

echo "== shared work: one ticket, every worktree =="
SW_WAR="$REPO_ROOT/${WAR#./}"
sw_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sw_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
SW_T=$(mktemp -d)
SW_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1
    "PATH=$(dirname "$SW_WAR"):$PATH")
sw_g() { "${SW_ENV[@]}" git "$@" >/dev/null 2>&1; }
# sw_as <actor> <worktree> <war args...>: stdout and stderr to $SW_T/out, $SW_T/err.
sw_as() {
    local actor="$1" at="$2"; shift 2
    (cd "$at" && "${SW_ENV[@]}" OPENWARRANT_ACTOR="$actor" timeout 120 "$SW_WAR" "$@" \
        >"$SW_T/out" 2>"$SW_T/err" </dev/null)
}
sw_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$SW_T/out"; }
SW_A="$SW_T/a" SW_B="$SW_T/b"

mkdir -p "$SW_A"
sw_g -C "$SW_A" init -q -b main
sw_as owner "$SW_A" init
sw_g -C "$SW_A" add -A
sw_g -C "$SW_A" commit -qm seed
sw_g -C "$SW_A" worktree add -q "$SW_B" -b feature
if [[ ! -f "$SW_B/openwarrant.toml" ]]; then
    printf 'PLANT SETUP FAILED: two worktrees: %s\n' "$(head -c 300 "$SW_T/err")" >&2; exit 9
fi
SW_REG="$SW_A/.git/openwarrant/tickets"

# --- created in A, shown at once in B ---------------------------------------------
sw_as bob "$SW_B" status; SW_BEFORE=$(grep -c '^Tickets: 0 open' "$SW_T/out")
sw_as alice "$SW_A" --json create "Rotate the signing keys" --item "write the runbook" --item "rotate"; SW_ID=$(sw_id)
sw_as bob "$SW_B" status; cp "$SW_T/out" "$SW_T/status"
sw_as bob "$SW_B" view prime; cp "$SW_T/out" "$SW_T/prime"
sw_as bob "$SW_B" view ready; cp "$SW_T/out" "$SW_T/ready"
if [[ $SW_BEFORE == 1 && -n "$SW_ID" && ! -e "$(tk "$SW_B/docs/tickets/$SW_ID.md")" ]] \
    && grep -q '^Tickets: 1 open' "$SW_T/status" \
    && grep -q "^- $SW_ID — Rotate the signing keys .*elsewhere: worktree $SW_A (branch main)" "$SW_T/prime" \
    && grep -q "^$SW_ID .*Rotate the signing keys.*\[elsewhere: worktree $SW_A (branch main)\]" "$SW_T/ready" \
    && grep -q "^  $SW_ID .*Rotate the signing keys.*\[elsewhere: worktree" "$SW_T/status"; then
    sw_ok "created in A, shown in B" "uncommitted in A; B's status counts it, prime and ready title it, marked elsewhere"
else
    sw_fail "created in A, shown in B" "before $SW_BEFORE id '$SW_ID': $(head -c 200 "$SW_T/status") | $(grep -a "$SW_ID" "$SW_T/prime" | head -c 300)"
fi

# --- an id the register holds is not minted again ---------------------------------
sw_as alice "$SW_A" --json create "A second, unrelated thing"; SW_ID2=$(sw_id)
SW_NAMES=$(find "$SW_REG" -maxdepth 1 -type f -name 't-*' ! -name '*~*' | wc -l)
if [[ -n "$SW_ID2" && "$SW_ID2" != "$SW_ID" && $SW_NAMES == 2 && -f "$SW_REG/$SW_ID" && -f "$SW_REG/$SW_ID2" ]]; then
    sw_ok "one id, one ticket" "each id is one name in the clone's register, taken as a claim takes a lock"
else
    sw_fail "one id, one ticket" "ids '$SW_ID' '$SW_ID2', $SW_NAMES entries"
fi

# --- .openwarrant/next.md in B names it too, marked ----------------------------------
sw_as bob "$SW_B" create "Something of B's own"; SW_RC=$?
if [[ $SW_RC == 0 ]] && grep -q "^$SW_ID .*Rotate the signing keys.*\[elsewhere: worktree $SW_A" "$SW_B/.openwarrant/next.md" \
    && ! grep -q "Something of B's own.*elsewhere" "$SW_B/.openwarrant/next.md"; then
    sw_ok "next.md lists it, marked" "B's .openwarrant/next.md offers A's ticket as elsewhere; B's own is not marked"
else
    sw_fail "next.md lists it, marked" "exit $SW_RC: $(head -c 400 "$SW_B/.openwarrant/next.md" 2>&1)"
fi

# --- unknown where nothing announces it ---------------------------------------------
sw_as bob "$SW_B" start t-0000zz; SW_RC=$?
if [[ $SW_RC != 0 ]] && grep -q 'ticket.unknown' "$SW_T/err"; then
    sw_ok "unannounced is unknown" "an id no worktree registered is refused in B (ticket.unknown)"
else
    sw_fail "unannounced is unknown" "exit $SW_RC: $(head -c 300 "$SW_T/err")"
fi

# --- worked in B: the document is brought there, and the branches merge ------------
sw_as bob "$SW_B" start "$SW_ID"; SW_RC=$?
SW_SAME=no; cmp -s "$(tk "$SW_A/docs/tickets/$SW_ID.md")" "$(tk "$SW_B/docs/tickets/$SW_ID.md")" && SW_SAME=yes
sw_as alice "$SW_A" plan add "$SW_ID" "tell the on-call"; SW_ADD=$?
SW_ITEM=$(grep -o 'i-[0-9a-f]*' "$(tk "$SW_B/docs/tickets/$SW_ID.md")" | head -1)
sw_as bob "$SW_B" done "$SW_ID/$SW_ITEM" --note "written"; SW_DONE=$?
sw_g -C "$SW_A" add -A; sw_g -C "$SW_A" commit -qm "a: an item added"
sw_g -C "$SW_B" add -A; sw_g -C "$SW_B" commit -qm "b: an item ticked"
(cd "$SW_A" && "${SW_ENV[@]}" git merge -q feature -m merge >"$SW_T/merge" 2>&1); SW_MERGE=$?
SW_DOC="$(tk "$SW_A/docs/tickets/$SW_ID.md")"
if [[ $SW_RC == 0 && $SW_SAME == yes && $SW_ADD == 0 && $SW_DONE == 0 && $SW_MERGE == 0 ]] \
    && grep -q '^- \[x\] write the runbook' "$SW_DOC" && grep -q '^- \[ \] tell the on-call' "$SW_DOC" \
    && ! grep -q '^<<<<<<<' "$SW_DOC"; then
    sw_ok "started in B, merged clean" "same bytes at the same path; A's added item and B's tick both survive the merge"
else
    sw_fail "started in B, merged clean" "start $SW_RC same $SW_SAME add $SW_ADD done $SW_DONE merge $SW_MERGE: $(head -c 200 "$SW_T/merge") | $(tail -5 "$SW_DOC" | head -c 300)"
fi

# --- closed in A: no longer offered in B --------------------------------------------
sw_as bob "$SW_B" view prime; SW_OPEN=$(grep -c "^- $SW_ID2 — A second, unrelated thing" "$SW_T/out")
sw_as alice "$SW_A" start "$SW_ID2"; sw_as alice "$SW_A" done "$SW_ID2"; SW_DONE=$?
sw_as bob "$SW_B" view prime; SW_GONE=$(grep -c "$SW_ID2" "$SW_T/out")
if [[ $SW_OPEN == 1 && $SW_DONE == 0 && $SW_GONE == 0 && ! -e "$SW_REG/$SW_ID2" ]]; then
    sw_ok "closed work is retired" "listed in B while open in A; done in A, its entry is dropped and B lists it no more"
else
    sw_fail "closed work is retired" "open $SW_OPEN done $SW_DONE still listed $SW_GONE: $(head -c 300 "$SW_T/err")"
fi

# --- one id, two tickets: flagged, not hidden ---------------------------------------
sw_as alice "$SW_A" --json create "Only A has this one"; SW_ID3=$(sw_id)
sw_as bob "$SW_B" view tickets; SW_CLEAN=$(grep -c 'ticket.id-collision' "$SW_T/err")
sed -e 's/^uuid: .*/uuid: 01a1d2f8-0000-7000-8000-000000000009/' -e 's/Only A has this one/A stranger under the same id/' \
    "$(tk "$SW_A/docs/tickets/$SW_ID3.md")" >"$(tk "$SW_B/docs/tickets/$SW_ID3.md")"
sw_as bob "$SW_B" view tickets; cp "$SW_T/err" "$SW_T/collision"; cp "$SW_T/out" "$SW_T/listed"
sw_as bob "$SW_B" status
if [[ $SW_CLEAN == 0 ]] && grep -q "ticket.id-collision.*$SW_ID3.*A stranger under the same id.*Only A has this one" "$SW_T/collision" \
    && grep -q "^$SW_ID3 .*A stranger under the same id" "$SW_T/listed" \
    && grep -q '1 id shared by two tickets' "$SW_T/out"; then
    sw_ok "an id collision is flagged" "no finding while the UUIDs agree; two UUIDs under one id name both, and the one here stays listed"
else
    sw_fail "an id collision is flagged" "clean $SW_CLEAN: $(head -c 300 "$SW_T/collision") | $(head -c 200 "$SW_T/out")"
fi

[[ -n "$SW_T" && "$SW_T" == */tmp.* ]] && /usr/bin/rm -rf "$SW_T"
