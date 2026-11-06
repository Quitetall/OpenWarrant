# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Work tracking under FORMAT-1.0 §11 (draft 3), on a scratch program (FT)
# with ten worktrees of one clone; nothing in this repository moves.
#
# Accepted: one of ten actors racing `war claim` on one item of a Format 1.0
# ticket, each in its own worktree, wins, and its claim is the one journalled
# (a v2 `work.claimed`); a write at the current revision lands; a journal
# holding v1 and v2 lines reads, and the next write extends it with neither
# rewritten; ten concurrent `war next --at` callers get byte-identical
# answers; `due` decides before priority; the critical path (the item more
# work waits on) goes first.
# Refused, by name: the nine other racers (work.claimed-by-other); a write
# whose `--if-rev` is stale (work.stale-revision), with the document and the
# journal byte-for-byte as they were; a journal line that does not parse
# (work.journal-malformed), and a line of a newer envelope read UNKNOWN
# (work.journal-newer); and, as the other half of each order, the order
# without the due date or the dependency.

echo "== work tracking: Format 1.0 claims, journals, next (FORMAT-1.0 §11) =="
FT_ROOT=$(scratch_corpus FT)
[[ -d "${FT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
FT_TMP=$(mktemp -d)
SCRATCH_CORPORA+=("$FT_ROOT" "$FT_TMP")
FT_WAR="$REPO_ROOT/${WAR#./}"
ft_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ft_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ft_war() { local at="$1"; shift; env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$FT_WAR" --root "$at" "$@" </dev/null; }
ft_commit() { git -C "$1" add -A >/dev/null 2>&1 && git -C "$1" -c user.email=plant@invalid -c user.name=plant commit -qm "$2" >/dev/null 2>&1; }
# ft_ticket <alias> <uuid> <extra frontmatter> <items>: a Format 1.0 ticket.
ft_ticket() {
    mkdir -p "$FT_ROOT/docs/tickets"
    printf -- '---\nformat: openwarrant/1.0\nprofile: ticket\nalias: %s\nuuid: %s\nsummary: Plant work %s.\n%b---\n# Plant work %s\n\n## Checklist\n\n%b' \
        "$1" "$2" "$1" "$3" "$1" "$4" > "$FT_ROOT/docs/tickets/$1.md"
}
FT_AT=2026-10-15T12:00:00Z
ft_order() { ft_war "$FT_ROOT" --json next --at "$FT_AT" 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)
print(" ".join(r["ticket"] + ("/" + r["item"] if r.get("item") else "") for r in v["result"].get("ready", [])))' 2>&1; }

# --- ten worktrees, one item, one winner ------------------------------------------
ft_ticket t-f001 01a0d342-ee5e-7b71-9623-f2d8cd5a0001 "" "- [ ] contested (i-f101)\n- [ ] guarded (i-f102)\n"
ft_commit "$FT_ROOT" "a Format 1.0 ticket"
for n in 0 1 2 3 4 5 6 7 8 9; do
    git -C "$FT_ROOT" worktree add -q "$FT_TMP/wt$n" >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: git worktree add %s\n' "$n" >&2; exit 9; }
done
FT_PIDS=()
for n in 0 1 2 3 4 5 6 7 8 9; do
    ( ft_war "$FT_TMP/wt$n" claim t-f001/i-f101 --as "racer-$n" >/dev/null 2>"$FT_TMP/err.$n"; echo $? >"$FT_TMP/rc.$n" ) &
    FT_PIDS+=($!)
done
for pid in "${FT_PIDS[@]}"; do wait "$pid"; done
FT_WON=""; FT_NAMED=0
for n in 0 1 2 3 4 5 6 7 8 9; do
    if [[ "$(cat "$FT_TMP/rc.$n")" == 0 ]]; then FT_WON="$FT_WON racer-$n"
    elif grep -q '^refused (work.claimed-by-other): ' "$FT_TMP/err.$n"; then FT_NAMED=$((FT_NAMED + 1)); fi
done
FT_WON=${FT_WON# }
# The winner's claim is journalled in its own worktree, in v2.
FT_WN=${FT_WON#racer-}
FT_J="$FT_TMP/wt$FT_WN/docs/tickets/t-f001/journal.jsonl"
FT_JOK=$(python3 -c '
import json, sys
lines = [json.loads(l) for l in open(sys.argv[1])]
ok = len(lines) == 1 and lines[0]["v"] == 2 and lines[0]["type"] == "work.claimed" \
    and lines[0]["actor_ref"] == sys.argv[2] and isinstance(lines[0]["payload"], dict) \
    and "document_uuid" in lines[0]
print("yes" if ok else lines)' "$FT_J" "$FT_WON" 2>&1)
if [[ -n "$FT_WON" && "$FT_WON" != *" "* && $FT_NAMED -eq 9 && "$FT_JOK" == yes ]]; then
    ft_ok "ten worktrees, one claim" "$FT_WON won; 9 refused work.claimed-by-other; one v2 work.claimed"
else
    ft_fail "ten worktrees, one claim" "won '$FT_WON', named $FT_NAMED, journal $FT_JOK; $(cat "$FT_TMP"/err.* | head -2)"
fi

# --- compare-and-set: a stale revision writes nothing -----------------------------------
FT_W="$FT_TMP/wt$FT_WN"
ft_war "$FT_W" claim t-f001/i-f102 --as "$FT_WON" >/dev/null 2>&1
FT_DOC_BEFORE=$(sha256sum "$FT_W/docs/tickets/t-f001.md" | cut -c1-64)
FT_J_BEFORE=$(sha256sum "$FT_J" | cut -c1-64)
FT_ERR=$(ft_war "$FT_W" done t-f001/i-f102 --as "$FT_WON" --if-rev "sha256:$(printf '0%.0s' {1..64})" 2>&1 >/dev/null); FT_S=$?
FT_DOC_AFTER=$(sha256sum "$FT_W/docs/tickets/t-f001.md" | cut -c1-64)
FT_J_AFTER=$(sha256sum "$FT_J" | cut -c1-64)
FT_REV=$(ft_war "$FT_W" --json show t-f001 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)
print([i["revision"] for i in v["result"]["items"] if i["id"] == "i-f102"][0])' 2>/dev/null)
ft_war "$FT_W" done t-f001/i-f102 --as "$FT_WON" --if-rev "$FT_REV" >/dev/null 2>&1; FT_S2=$?
if [[ $FT_S -eq 2 && "$FT_DOC_BEFORE" == "$FT_DOC_AFTER" && "$FT_J_BEFORE" == "$FT_J_AFTER" && $FT_S2 -eq 0 ]] \
    && grep -q '^refused (work.stale-revision): ' <<<"$FT_ERR" && grep -q 'and it is now sha256:' <<<"$FT_ERR" \
    && grep -q '^- \[x\] guarded (i-f102) — done by ' "$FT_W/docs/tickets/t-f001.md"; then
    ft_ok "a stale revision writes nothing" "work.stale-revision naming the current one; document and journal unchanged; the current revision lands"
else
    ft_fail "a stale revision writes nothing" "stale exit $FT_S, doc moved $([[ "$FT_DOC_BEFORE" != "$FT_DOC_AFTER" ]] && echo yes), journal moved $([[ "$FT_J_BEFORE" != "$FT_J_AFTER" ]] && echo yes), current exit $FT_S2 ($FT_REV): $FT_ERR"
fi

# --- one journal, v1 and v2 lines ------------------------------------------------------
# A v1 line (as a journal written before Format 1.0 holds) ahead of the v2
# lines: it reads as `work.note_added`, the next write extends the file and
# rewrites neither.
FT_V1=$(python3 -c '
import hashlib, json
u, t, p = "01a0d342-ee5e-7b71-9623-f2d8cd5a0001", "ticket.note_added", json.dumps({"note": "from before"}, separators=(",", ":"))
print(json.dumps({"v": 1, "id": "01a0d342-ee5e-7b71-9623-f2d8cd5a0fff", "warrant_uuid": u, "type": t, "class": "draft_history",
    "actor_ref": "history", "occurred_at": "2026-10-01T00:00:00Z", "payload": p,
    "idempotency_key": hashlib.sha256(f"{u}\n{t}\n{p}".encode()).hexdigest()}, separators=(",", ":")))')
{ printf '%s\n' "$FT_V1"; cat "$FT_J"; } > "$FT_TMP/mixed.jsonl" && cp "$FT_TMP/mixed.jsonl" "$FT_J"
FT_MIXED=$(sha256sum "$FT_J" | cut -c1-64)
ft_war "$FT_W" note t-f001 "after the mix" --as "$FT_WON" >/dev/null 2>&1; FT_S=$?
FT_PREFIX=$(head -c "$(stat -c %s "$FT_TMP/mixed.jsonl")" "$FT_J" | sha256sum | cut -c1-64)
FT_CHECK=$(ft_war "$FT_W" check 2>&1)
FT_SHAPES=$(python3 -c '
import json, sys
print(" ".join(str(json.loads(l)["v"]) for l in open(sys.argv[1])))' "$FT_J" 2>&1)
printf '{"v":2,"payload":"not an object"}\n' >> "$FT_J"
FT_BAD=$(ft_war "$FT_W" check 2>&1); FT_BS=$?
# A line of an envelope newer than v2 is UNKNOWN, never malformed.
head -n -1 "$FT_J" > "$FT_TMP/j" && cp "$FT_TMP/j" "$FT_J"
sed -n 1p "$FT_J" | sed 's/"v":1/"v":3/' >> "$FT_J"
FT_NEW=$(ft_war "$FT_W" check 2>&1); FT_NS=$?
if [[ $FT_S -eq 0 && "$FT_PREFIX" == "$FT_MIXED" && "$FT_SHAPES" == "1 2 2 2 2" && $FT_BS -ne 0 && $FT_NS -ne 0 ]] \
    && ! grep -q 'work.journal' <<<"$FT_CHECK" && grep -qE 'ERROR +work\.journal-malformed' <<<"$FT_BAD" \
    && grep -qE 'UNKNOWN +work\.journal-newer' <<<"$FT_NEW" && ! grep -q 'work.journal-malformed' <<<"$FT_NEW"; then
    ft_ok "v1 and v2 in one journal" "read and extended, nothing rewritten (v $FT_SHAPES); a bad line refused work.journal-malformed; a v3 line UNKNOWN work.journal-newer"
else
    ft_fail "v1 and v2 in one journal" "note $FT_S, prefix kept $([[ "$FT_PREFIX" == "$FT_MIXED" ]] && echo yes), shapes '$FT_SHAPES', bad exit $FT_BS, newer exit $FT_NS: $(grep 'work.journal' <<<"$FT_CHECK$FT_BAD$FT_NEW" | head -3 | tr '\n' '|')"
fi

# --- next: one answer at one time ---------------------------------------------------------
# The main worktree's copy of t-f001 is not the one worked above; it leaves.
rm -f "$FT_ROOT/docs/tickets/t-f001.md"
ft_ticket t-f002 01a0d342-ee5e-7b71-9623-f2d8cd5a0002 "priority: 0\n" "- [ ] urgent by priority (i-f201)\n"
ft_ticket t-f003 01a0d342-ee5e-7b71-9623-f2d8cd5a0003 "priority: 3\ndue: 2026-11-01\n" "- [ ] due soon (i-f301)\n"
FT_PIDS=()
for n in 0 1 2 3 4 5 6 7 8 9; do
    ( ft_war "$FT_ROOT" --json next --at "$FT_AT" >"$FT_TMP/next.$n" 2>/dev/null ) &
    FT_PIDS+=($!)
done
for pid in "${FT_PIDS[@]}"; do wait "$pid"; done
FT_SAME=1
for n in 1 2 3 4 5 6 7 8 9; do cmp -s "$FT_TMP/next.0" "$FT_TMP/next.$n" || FT_SAME=0; done
if [[ $FT_SAME -eq 1 && -s "$FT_TMP/next.0" ]] && grep -q '"ready"' "$FT_TMP/next.0"; then
    ft_ok "ten callers, one answer" "war next --at $FT_AT: ten concurrent answers byte-identical"
else
    ft_fail "ten callers, one answer" "the answers differ or are empty: $(head -c 200 "$FT_TMP/next.0")"
fi

# --- next: due before priority, the critical path first ---------------------------------
FT_DUE=$(ft_order)
ft_ticket t-f003 01a0d342-ee5e-7b71-9623-f2d8cd5a0003 "priority: 3\n" "- [ ] due soon (i-f301)\n"
FT_NODUE=$(ft_order)
if [[ "$FT_DUE" == "t-f003/i-f301 t-f002/i-f201" && "$FT_NODUE" == "t-f002/i-f201 t-f003/i-f301" ]]; then
    ft_ok "due beats priority" "with due: $FT_DUE; without: $FT_NODUE"
else
    ft_fail "due beats priority" "with due '$FT_DUE'; without '$FT_NODUE'"
fi
rm -f "$FT_ROOT/docs/tickets/t-f002.md" "$FT_ROOT/docs/tickets/t-f003.md"
ft_ticket t-f004 01a0d342-ee5e-7b71-9623-f2d8cd5a0004 "" "- [ ] alone (i-f401)\n- [ ] first (i-f402)\n- [ ] then (i-f403, after i-f402)\n"
FT_PATH=$(ft_order)
ft_ticket t-f004 01a0d342-ee5e-7b71-9623-f2d8cd5a0004 "" "- [ ] alone (i-f401)\n- [ ] first (i-f402)\n- [ ] then (i-f403)\n"
FT_FLAT=$(ft_order)
if [[ "$FT_PATH" == "t-f004/i-f402 t-f004/i-f401" && "$FT_FLAT" == "t-f004/i-f401 t-f004/i-f402 t-f004/i-f403" ]]; then
    ft_ok "the critical path first" "i-f403 waits on i-f402: $FT_PATH; with no wait, position: $FT_FLAT"
else
    ft_fail "the critical path first" "with the wait '$FT_PATH'; without '$FT_FLAT'"
fi

for n in 0 1 2 3 4 5 6 7 8 9; do git -C "$FT_ROOT" worktree remove --force "$FT_TMP/wt$n" >/dev/null 2>&1; done
