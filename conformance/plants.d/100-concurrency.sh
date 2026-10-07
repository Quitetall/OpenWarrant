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
