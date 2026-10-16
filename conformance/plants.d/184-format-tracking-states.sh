# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# FORMAT-1.0 §11 (draft 3, revision 1) as the conformance corpus reads it, on
# a scratch program (FS); nothing in this repository moves.
#
# Accepted: `war plan states --at` reports each state by its family of §11.1
# (a document's `phase`, an item's one `checklist` state, a claimed item
# `claimed`, an item waiting on it `blocked`); a note on a Format 1.0 document
# is one `work.note_added` event `{target, note}` with the document unchanged,
# and the same note from the same actor replays, writing nothing; a done based
# on the current revision (`--if-rev`) lands without a prior claim; done,
# reopened and done again are three events, `episode` 0, 0 and 1.
# Refused, by name and line: the same note from another actor
# (work.idempotency-conflict at the journal line that records it); an
# unterminated last journal line, though it parses (work.journal-torn at that
# line); a done whose base is stale (work.stale-revision at the item's line);
# a claim on the document while another actor holds one of its items
# (work.claimed-by-other, naming that item's claim file); a done suffix with
# the tick level `[claimed]` (format.item at its line).

echo "== work tracking: states, notes, episodes, lines (FORMAT-1.0 §11, §13.1) =="
FS_ROOT=$(scratch_corpus FS)
[[ -d "${FS_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
FS_WAR="$REPO_ROOT/${WAR#./}"
fs_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fs_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
fs_war() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$FS_WAR" --root "$FS_ROOT" "$@" </dev/null; }
fs_commit() { git -C "$FS_ROOT" add -A >/dev/null 2>&1 && git -C "$FS_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1; }
FS_AT=2026-10-14T10:00:00Z
FS_DOC="$FS_ROOT/docs/tickets/t-0d61d.md"
FS_J="$FS_ROOT/docs/tickets/t-0d61d/journal.jsonl"
# fs_ticket <items>: the corpus's ticket t-0d61d with these checklist lines.
fs_ticket() {
    mkdir -p "$FS_ROOT/docs/tickets"
    printf -- '---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-0d61d\nuuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10\nsummary: Password reset by email.\n---\n# Password reset by email\n\n## Checklist\n%b' "$1" > "$FS_DOC"
}
# fs_rules <prefix>: `<rule> <file>` of each diagnostic of stdin's envelope.
fs_rules() { python3 -c '
import json, sys
v = json.load(sys.stdin)
print(" ".join(sorted(d["rule"] + " " + (d.get("file") or "") for d in v.get("diagnostics", [])
                      if d["rule"].startswith(sys.argv[1]))))' "$1" 2>&1; }
fs_states() { fs_war --json plan states --at "$FS_AT" 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)
print(" ".join(f"{s[\"record\"]}:{s[\"kind\"]}:{s[\"value\"]}" for s in v["result"]["states"]))' 2>&1; }

fs_ticket "- [ ] Reset endpoint (i-9c01)\n- [ ] Email template (i-4d11)\n- [ ] Docs (i-77be, after i-4d11)\n"
fs_commit "the corpus's ticket"

# --- states by family; a document claim refused while an item is held ----------------
fs_war claim t-0d61d/i-4d11 --as bob --at 2026-10-14T09:50:00Z >/dev/null 2>&1; FS_S=$?
FS_STATES=$(fs_states)
FS_WHOLE=$(fs_war --json claim t-0d61d --as alice --at "$FS_AT" 2>/dev/null | fs_rules work.)
FS_WANT="t-0d61d:phase:in_progress t-0d61d/i-4d11:checklist:claimed t-0d61d/i-77be:checklist:blocked t-0d61d/i-9c01:checklist:open"
if [[ $FS_S -eq 0 && "$FS_STATES" == "$FS_WANT" && "$FS_WHOLE" == "work.claimed-by-other "*"openwarrant/claims/t-0d61d--i-4d11.lock" ]]; then
    fs_ok "states by family" "$FS_STATES; the whole document refused naming bob's item claim"
else
    fs_fail "states by family" "claim $FS_S; states '$FS_STATES'; whole '$FS_WHOLE'"
fi

# --- a note: one event, once ------------------------------------------------------
FS_DOC_BEFORE=$(sha256sum "$FS_DOC" | cut -c1-64)
fs_war note t-0d61d "the template needs a plain-text part" --as bob --at "$FS_AT" >/dev/null 2>&1; FS_S1=$?
FS_J1=$(sha256sum "$FS_J" | cut -c1-64)
fs_war note t-0d61d "the template needs a plain-text part" --as bob >/dev/null 2>&1; FS_S2=$?
FS_J2=$(sha256sum "$FS_J" | cut -c1-64)
FS_OTHER=$(fs_war --json note t-0d61d "the template needs a plain-text part" --as alice 2>/dev/null | fs_rules work.)
FS_J3=$(sha256sum "$FS_J" | cut -c1-64)
FS_LINE=$(grep -n '"work.note_added"' "$FS_J" | cut -d: -f1)
FS_PAYLOAD=$(sed -n "${FS_LINE}p" "$FS_J" | python3 -c 'import json, sys; print(json.dumps(json.load(sys.stdin)["payload"], sort_keys=True))')
if [[ $FS_S1 -eq 0 && $FS_S2 -eq 0 && "$FS_J1" == "$FS_J2" && "$FS_J2" == "$FS_J3" \
    && "$(sha256sum "$FS_DOC" | cut -c1-64)" == "$FS_DOC_BEFORE" \
    && "$FS_PAYLOAD" == '{"note": "the template needs a plain-text part", "target": "t-0d61d"}' \
    && "$FS_OTHER" == "work.idempotency-conflict docs/tickets/t-0d61d/journal.jsonl:$FS_LINE" ]]; then
    fs_ok "a note is written once" "one work.note_added {target, note}, document unchanged; bob's replay wrote nothing; alice's refused at journal line $FS_LINE"
else
    fs_fail "a note is written once" "note $FS_S1/$FS_S2, payload $FS_PAYLOAD, other '$FS_OTHER' (line $FS_LINE)"
fi

# --- a based done without a claim; a stale base refused at the item's line -------------
FS_STALE=$(fs_war --json done t-0d61d/i-9c01 --as carol --if-rev "sha256:$(printf '0%.0s' {1..64})" 2>/dev/null | fs_rules work.)
FS_DOC_STALE=$(sha256sum "$FS_DOC" | cut -c1-64)
FS_REV=$(fs_war --json show t-0d61d 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)
print([i["revision"] for i in v["result"]["items"] if i["id"] == "i-9c01"][0])' 2>/dev/null)
fs_war done t-0d61d/i-9c01 --as carol --if-rev "$FS_REV" --at "$FS_AT" >/dev/null 2>&1; FS_S=$?
if [[ "$FS_STALE" == "work.stale-revision docs/tickets/t-0d61d.md:11" && "$FS_DOC_STALE" == "$FS_DOC_BEFORE" && $FS_S -eq 0 ]] \
    && grep -q '^- \[x\] Reset endpoint (i-9c01) — done by carol, 2026-10-14$' "$FS_DOC"; then
    fs_ok "a based done needs no claim" "stale base refused at t-0d61d.md:11, nothing written; the current base ticks i-9c01"
else
    fs_fail "a based done needs no claim" "stale '$FS_STALE'; current exit $FS_S ($FS_REV): $(grep i-9c01 "$FS_DOC")"
fi

# --- episode: done, reopened, done again are three events ------------------------------
fs_war admin reopen t-0d61d/i-9c01 --as carol --reason regressed >/dev/null 2>&1
fs_war claim t-0d61d/i-9c01 --as carol >/dev/null 2>&1
fs_war done t-0d61d/i-9c01 --as carol >/dev/null 2>&1
FS_EP=$(python3 -c '
import json, sys
evs = [json.loads(l) for l in open(sys.argv[1])]
print(" ".join(f"{e[\"type\"].split(\".\")[1]}={e[\"payload\"].get(\"episode\")}" for e in evs
               if e["payload"].get("target") == "t-0d61d/i-9c01"))' "$FS_J" 2>&1)
if [[ "$FS_EP" == "claimed=0 item_done=0 item_reopened=0 claimed=1 item_done=1" ]]; then
    fs_ok "episode counts again" "$FS_EP"
else
    fs_fail "episode counts again" "'$FS_EP'"
fi

# --- a torn journal: the unterminated last line, though it parses ----------------------
fs_commit "worked"
FS_N=$(wc -l < "$FS_J")
FS_OK_CHECK=$(fs_war --json plan states --at "$FS_AT" 2>/dev/null | fs_rules work.journal)
head -c -1 "$FS_J" > "$FS_J.tmp" && cp "$FS_J.tmp" "$FS_J" && rm -f "$FS_J.tmp"
FS_TORN=$(fs_war --json plan states --at "$FS_AT" 2>/dev/null | fs_rules work.journal)
if [[ -z "$FS_OK_CHECK" && "$FS_TORN" == "work.journal-torn docs/tickets/t-0d61d/journal.jsonl:$FS_N" ]]; then
    fs_ok "an unterminated line is torn" "every line ended: read; the last line's newline cut: torn at line $FS_N"
else
    fs_fail "an unterminated line is torn" "whole '$FS_OK_CHECK'; cut '$FS_TORN'"
fi
printf '\n' >> "$FS_J"

# --- the done suffix's tick level -------------------------------------------------------
fs_ticket "- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12 [observed]\n"
FS_OBS=$(fs_war --json check 2>/dev/null | fs_rules format.item)
fs_ticket "- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12 [claimed]\n"
FS_CLM=$(fs_war --json check 2>/dev/null | fs_rules format.item)
if [[ -z "$FS_OBS" && "$FS_CLM" == "format.item docs/tickets/t-0d61d.md:11" ]]; then
    fs_ok "a tick level is a level" "[observed] reads; [claimed] refused format.item at line 11"
else
    fs_fail "a tick level is a level" "observed '$FS_OBS'; claimed '$FS_CLM'"
fi
