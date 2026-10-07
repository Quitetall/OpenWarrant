# shellcheck shell=bash
# OW-WAR-0148 M5 (OBL-009, OBL-010, OBL-011) — tickets on the kernel, ticket
# features, and GitHub in and out.
#
# One scratch program (TF) with the committed ticket profile. Five planted
# tickets: an epic E; A (bug; backend, auth; part of E); B (feature; ui; part
# of E); C (chore; docs); D (no type, no label; "flaky cache" in its body,
# "invalidate" in an item's done note).
#
# Accepted: `war tickets --type/--label/--state/--text/--search/--epic` each
# return exactly the planted set; `war show <epic>` lists its tickets and its
# progress over them, which moves when one is done, and `war ready` never
# offers the epic whole; items are records in `war model`, a blocker a
# `depends_on`, the epic link a `part_of`; a ticket written before M5 reads,
# claims and finishes with its files moving only in the line the act wrote,
# and `war edit` adds exactly one manifest line. With a fake `gh` on PATH
# that records its argv and environment: `war create --issue 12` makes a
# ticket from one read; finishing it without `[intake.writeback]` runs
# nothing more; with it, finishing runs exactly one comment and one close with
# the configured argv, in that order.
# Refused, each by rule: a filter naming an unknown state
# (ticket.filter-state-unknown) or type (ticket.filter-type-unknown); a label
# outside a closed set, on create, edit and filter (ticket.label-unknown),
# nothing written; a blocked item's claim (ticket.blocked); a `fetch_argv`
# that writes (intake.fetch-not-a-read) before any process starts; and a
# write that fails leaves the ticket done and reports the issue UNKNOWN
# (ticket.issue-unknown), the close never run.

echo "== ticket features (OW-WAR-0148 M5) =="
TF_ROOT=$(scratch_corpus TF)
[[ -d "${TF_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
TF_TMP=$(mktemp -d)
TF_WAR="$REPO_ROOT/${WAR#./}"
tf_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tf_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
tfw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID PATH="$TF_TMP/bin:$PATH" \
    FAKE_GH_LOG="$TF_TMP/gh.log" FAKE_GH_MARK=passed-through "$TF_WAR" --root "$TF_ROOT" "$@" </dev/null; }
tfj() { tfw --json "$@" 2>/dev/null; }
tf_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
# The ids `war tickets <filters>` lists, sorted, comma-joined.
tf_set() { tf_field "$(tfj tickets "$@")" '",".join(sorted(t["id"] for t in v["result"]["tickets"]))'; }
tf_sorted() { tr ',' '\n' <<<"$1" | sort | paste -sd, -; }
tf_id() { tf_field "$1" 'v["result"]["id"]'; }
tf_item() { tf_field "$1" 'v["result"]["items"]['"$2"']["id"]'; }
tf_has() { grep -qF -- "$1" <<<"$2"; }
tf_line() { grep -qE -- "$1" <<<"$2"; }

# The fake `gh`: every call appends its argv and the one variable `war` must
# pass through untouched; `view` answers an issue, `comment` fails when told.
mkdir -p "$TF_TMP/bin"
cat > "$TF_TMP/bin/gh" <<'SH'
#!/usr/bin/env bash
printf 'argv=%s env=%s\n' "$*" "${FAKE_GH_MARK:-}" >> "$FAKE_GH_LOG"
case "$1 $2" in
  "issue view")
    printf '{"number":%s,"title":"Login fails on an empty password","body":"Steps: submit the form empty.","url":"https://x-access-token:secret@github.com/o/r/issues/%s?t=1"}\n' "$3" "$3";;
  "issue comment") [[ -f "$FAKE_GH_LOG.fail" ]] && { echo "HTTP 403: denied" >&2; exit 1; }; exit 0;;
  "issue close") exit 0;;
  *) exit 3;;
esac
SH
chmod +x "$TF_TMP/bin/gh"
: > "$TF_TMP/gh.log"

# ---------------------------------------------------------------- the plant --
TF_E=$(tf_id "$(tfj create "Password reset epic" --type epic)")
TF_OUT=$(tfj create "Fix the login crash" --type bug -l backend -l auth --part-of "$TF_E" -i "Reproduce it" -i "Guard the null")
TF_A=$(tf_id "$TF_OUT"); TF_A1=$(tf_item "$TF_OUT" 0); TF_A2=$(tf_item "$TF_OUT" 1)
TF_OUT=$(tfj create "Reset page" --type feature -l ui --part-of "$TF_E" -i "Draw the form")
TF_B=$(tf_id "$TF_OUT"); TF_B1=$(tf_item "$TF_OUT" 0)
TF_OUT=$(tfj create "Reset docs" --type chore -l docs -i "Explain the reset")
TF_C=$(tf_id "$TF_OUT"); TF_C1=$(tf_item "$TF_OUT" 0)
TF_OUT=$(tfj create "Speed up sessions" --body "The flaky cache drops sessions." -i "Measure")
TF_D=$(tf_id "$TF_OUT"); TF_D1=$(tf_item "$TF_OUT" 0)
tfw add "$TF_A" "Ship the fix" --after "$TF_A2" >/dev/null 2>&1
TF_A3=$(sed -n 's/^- \[ \] Ship the fix (\(i-[0-9a-f]*\), after .*)$/\1/p' "$TF_ROOT/docs/tickets/$TF_A/atoms/15-checklist.md")
tfw claim "$TF_C/$TF_C1" >/dev/null 2>&1 && tfw done "$TF_C/$TF_C1" >/dev/null 2>&1
tfw claim "$TF_D/$TF_D1" >/dev/null 2>&1 && tfw done "$TF_D/$TF_D1" --note "invalidate on logout" >/dev/null 2>&1
tfw claim "$TF_B/$TF_B1" >/dev/null 2>&1
tfw state "$TF_B/$TF_B1" in_review >/dev/null 2>&1
if [[ -z "$TF_E" || -z "$TF_A" || -z "$TF_B" || -z "$TF_C" || -z "$TF_D" || -z "$TF_A3" ]]; then
    printf 'PLANT SETUP FAILED: tickets E=%s A=%s B=%s C=%s D=%s A3=%s\n' "$TF_E" "$TF_A" "$TF_B" "$TF_C" "$TF_D" "$TF_A3" >&2
    exit 9
fi

# 1. Filters and search return exactly the planted sets.
TF_BAD=""
tf_want() {  # <label> <expected ids, comma-separated> <filters...>
    local label=$1 want got
    want=$(tf_sorted "$2"); shift 2
    got=$(tf_set "$@")
    [[ "$got" == "$want" ]] || TF_BAD="$TF_BAD [$label: got '$got' want '$want']"
}
tf_want type-bug "$TF_A" --type bug
tf_want type-epic "$TF_E" --type epic
tf_want label-backend "$TF_A" --label backend
tf_want label-both "$TF_A" --label backend --label auth
tf_want label-mixed "" --label backend --label ui
tf_want label-ui "$TF_B" -l ui
tf_want state-done "$TF_C,$TF_D" --state done
tf_want state-progress "$TF_B" --state in_progress
tf_want state-open "$TF_E,$TF_A" --state open
tf_want state-review "$TF_B" --state in_review
tf_want text "$TF_D" --text "FLAKY cache"
tf_want text-none "" --text "cache flaky"
tf_want search "$TF_D" --search "cache flak"
tf_want search-note "$TF_D" --search "invalid"
tf_want search-items "$TF_A" --search "guard null"
tf_want search-title "$TF_B,$TF_C,$TF_E" --search reset
tf_want epic "$TF_A,$TF_B" --epic "$TF_E"
tf_want combined "$TF_B" --epic "$TF_E" --state in_progress --search draw
TF_ALL=$(tf_set)
TF_ERR1=$(tfw tickets --state blocked 2>&1 >/dev/null); TF_S1=$?
TF_ERR2=$(tfw tickets --type story 2>&1 >/dev/null); TF_S2=$?
if [[ -z "$TF_BAD" && "$TF_ALL" == "$(tf_sorted "$TF_E,$TF_A,$TF_B,$TF_C,$TF_D")" && $TF_S1 -eq 2 && $TF_S2 -eq 2 ]] \
    && tf_has 'ticket.filter-state-unknown' "$TF_ERR1" && tf_has 'open, in_progress, done, in_review' "$TF_ERR1" \
    && tf_has 'ticket.filter-type-unknown' "$TF_ERR2"; then
    tf_ok "filters return exact sets" "type, label(s), state (fixed and in_review), text, search (title, intent, items, notes), epic: 18 sets exact; unknown state and type refused"
else
    tf_fail "filters return exact sets" "mismatch:$TF_BAD; all '$TF_ALL'; state $TF_S1 '$TF_ERR1'; type $TF_S2 '$TF_ERR2'"
fi

# 2. An epic lists its tickets and its progress over them; finishing one
#    moves it; `war ready` never offers the epic whole.
TF_SHOW0=$(tfw show "$TF_E" 2>/dev/null)
TF_P0=$(tf_field "$(tfj show "$TF_E")" '"%d/%d" % (v["result"]["ticket"]["tickets"]["done"], v["result"]["ticket"]["tickets"]["total"])')
tfw claim "$TF_B/$TF_B1" >/dev/null 2>&1; tfw done "$TF_B/$TF_B1" --note "form drawn" >/dev/null 2>&1
TF_SHOW1=$(tfw show "$TF_E" 2>/dev/null)
TF_P1=$(tf_field "$(tfj show "$TF_E")" '"%d/%d" % (v["result"]["ticket"]["tickets"]["done"], v["result"]["ticket"]["tickets"]["total"])')
TF_READY=$(tf_field "$(tfj ready)" '",".join(r["ticket"] + ("/" + r["item"] if r["item"] else "") for r in v["result"]["ready"])')
TF_LS=$(tfw tickets 2>/dev/null)
if [[ "$TF_P0" == "0/2" && "$TF_P1" == "1/2" ]] && tf_has "## Tickets (0/2 done)" "$TF_SHOW0" \
    && tf_has "## Tickets (1/2 done)" "$TF_SHOW1" \
    && tf_has "- [x] $TF_B — Reset page · done · 1/1 done" "$TF_SHOW1" \
    && tf_has "- [ ] $TF_A — Fix the login crash · open · 0/3 done" "$TF_SHOW1" \
    && [[ ",$TF_READY," != *",$TF_E,"* && ",$TF_READY," == *",$TF_A/$TF_A1,"* ]] \
    && tf_line "^$TF_E .*\[epic: 1/2 done\]" "$TF_LS"; then
    tf_ok "an epic lists its tickets and progress" "$TF_E: 0/2 → 1/2 when $TF_B is done; never offered whole by war ready"
else
    tf_fail "an epic lists its tickets and progress" "progress $TF_P0 → $TF_P1; ready '$TF_READY'; show: $(head -c 400 <<<"$TF_SHOW1")"
fi

# 3. The kernel's reading: items are records, a blocker is `depends_on`, the
#    epic link `part_of`; the store reads the same relation, so the blocked
#    item is refused claim.
TF_MODEL=$(tfj model | python3 -c '
import json, sys
m = json.load(sys.stdin)["result"]
types = {r["id"]: r["type"] for r in m["records"]}
for r in m["relations"]:
    if r["from"].startswith("t-"):
        print(r["from"], r["kind"], r["to"])
for i, t in types.items():
    if t in ("item", "ticket"):
        print("record", i, t)
' 2>&1)
TF_ERR=$(tfw claim "$TF_A/$TF_A3" 2>&1 >/dev/null); TF_S=$?
if tf_line "^$TF_A/$TF_A3 depends_on $TF_A/$TF_A2\$" "$TF_MODEL" \
    && tf_line "^$TF_A part_of $TF_E\$" "$TF_MODEL" \
    && tf_line "^record $TF_A/$TF_A1 item\$" "$TF_MODEL" \
    && [[ $TF_S -eq 2 ]] && tf_has 'ticket.blocked' "$TF_ERR" && tf_has "waits on $TF_A2" "$TF_ERR"; then
    tf_ok "tickets read through the kernel" "items are records; $TF_A3 depends_on $TF_A2 and its claim is refused (ticket.blocked); $TF_A part_of $TF_E"
else
    tf_fail "tickets read through the kernel" "claim $TF_S '$TF_ERR'; model: $(head -c 400 <<<"$TF_MODEL")"
fi

# 4. A ticket written before M5 (its manifest as OW-WAR-0147 wrote it) reads,
#    claims and finishes; only the ticked line moves. `war edit` adds one
#    line to its manifest and moves nothing else.
TF_OLD="$TF_ROOT/docs/tickets/t-01d5"
mkdir -p "$TF_OLD/atoms"
cat > "$TF_OLD/manifest.toml" <<'TOML'
# A ticket (OW-WAR-0147): the working form of a delivery Warrant. The atoms beside
# this file are the ticket; `war show t-01d5` renders them. Nothing here is signed.
schema = "oh.war/ticket/v1"
id = "t-01d5"
uuid = "01a0f3c2-0000-7000-8000-000000000001"
title = "A ticket from before M5"
profile = "ticket"
priority = 1
created_at = "2026-09-26T10:00:00Z"
created_by = "someone"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"

[[atoms]]
ordinal = 15
role = "ticket.checklist"
path = "atoms/15-checklist.md"
TOML
printf '# A ticket from before M5\n\nWritten by hand.\n' > "$TF_OLD/atoms/10-intent.md"
printf '# Checklist\n\n- [ ] Keep working (i-0a01)\n- [x] Already ticked by hand (i-0a02)\n' > "$TF_OLD/atoms/15-checklist.md"
\cp -f "$TF_OLD/manifest.toml" "$TF_TMP/old.manifest"; \cp -f "$TF_OLD/atoms/15-checklist.md" "$TF_TMP/old.checklist"
tfw claim t-01d5/i-0a01 >/dev/null 2>&1; TF_S1=$?
tfw done t-01d5/i-0a01 --note "kept" >/dev/null 2>&1; TF_S2=$?
TF_MDIFF=$(cmp "$TF_TMP/old.manifest" "$TF_OLD/manifest.toml" 2>&1)
TF_CDIFF=$(diff "$TF_TMP/old.checklist" "$TF_OLD/atoms/15-checklist.md" | grep -c '^[<>]')
TF_CLINE=$(sed -n 3p "$TF_OLD/atoms/15-checklist.md")
TF_ST=$(tf_field "$(tfj tickets)" '[t["state"] for t in v["result"]["tickets"] if t["id"] == "t-01d5"][0]')
tfw edit t-01d5 --type chore >/dev/null 2>&1; TF_S3=$?
TF_EDIFF=$(diff "$TF_TMP/old.manifest" "$TF_OLD/manifest.toml")
if [[ $TF_S1 -eq 0 && $TF_S2 -eq 0 && $TF_S3 -eq 0 && -z "$TF_MDIFF" && "$TF_CDIFF" == 2 && "$TF_ST" == done ]] \
    && [[ "$TF_CLINE" =~ ^-\ \[x\]\ Keep\ working\ \(i-0a01\)\ —\ done\ by\ claude,\ [0-9-]+:\ kept$ ]] \
    && [[ "$TF_EDIFF" == $'9a10\n> type = "chore"' ]]; then
    tf_ok "a pre-M5 ticket works unchanged" "claimed and finished: manifest byte-identical, one checklist line changed; war edit added exactly 'type = \"chore\"'"
else
    tf_fail "a pre-M5 ticket works unchanged" "claim $TF_S1 done $TF_S2 edit $TF_S3; manifest '$TF_MDIFF'; checklist lines $TF_CDIFF '$TF_CLINE'; state $TF_ST; edit diff '$TF_EDIFF'"
fi

# 5. A closed label set: an unknown label is refused by rule on create, edit
#    and filter, and nothing is written; a declared one is admitted.
mkdir -p "$TF_ROOT/profiles"
python3 - "$REPO_ROOT/profiles/ticket.toml" "$TF_ROOT/profiles/ticket.toml" <<'PY'
import sys
src, dst = sys.argv[1:]
s = open(src).read()
old = 'labels = []\nlabels_closed = false\n'
assert s.count(old) == 1, "the shipped [fields] block moved"
open(dst, "w").write(s.replace(old, 'labels = ["backend", "ui", "auth", "docs"]\nlabels_closed = true\n'))
PY
TF_N0=$(find "$TF_ROOT/docs/tickets" -name manifest.toml | wc -l)
\cp -f "$TF_ROOT/docs/tickets/$TF_A/manifest.toml" "$TF_TMP/a.manifest"
TF_ERR1=$(tfw create "Labelled wrong" -l backend -l frontend 2>&1 >/dev/null); TF_S1=$?
TF_ERR2=$(tfw edit "$TF_A" -l frontend 2>&1 >/dev/null); TF_S2=$?
TF_ERR3=$(tfw tickets --label frontend 2>&1 >/dev/null); TF_S3=$?
TF_N1=$(find "$TF_ROOT/docs/tickets" -name manifest.toml | wc -l)
TF_ADIFF=$(cmp "$TF_TMP/a.manifest" "$TF_ROOT/docs/tickets/$TF_A/manifest.toml" 2>&1)
TF_GOOD=$(tf_id "$(tfj create "Labelled right" -l ui)")
command rm -rf "$TF_ROOT/profiles/ticket.toml" "$TF_ROOT/docs/tickets/$TF_GOOD"
if [[ $TF_S1 -eq 2 && $TF_S2 -eq 2 && $TF_S3 -eq 2 && "$TF_N0" == "$TF_N1" && -z "$TF_ADIFF" && -n "$TF_GOOD" ]] \
    && tf_has 'ticket.label-unknown' "$TF_ERR1" && tf_has '"frontend" is not in the ticket profile'"'"'s closed label set' "$TF_ERR1" \
    && tf_has 'ticket.label-unknown' "$TF_ERR2" && tf_has 'ticket.label-unknown' "$TF_ERR3"; then
    tf_ok "a closed label set refuses others" "frontend refused on create, edit and filter (ticket.label-unknown), nothing written; ui admitted"
else
    tf_fail "a closed label set refuses others" "create $TF_S1 '$TF_ERR1'; edit $TF_S2 '$TF_ERR2'; filter $TF_S3 '$TF_ERR3'; tickets $TF_N0/$TF_N1; manifest '$TF_ADIFF'; good '$TF_GOOD'"
fi

# 6. GitHub in: `war create --issue 12` makes a ticket from exactly one read,
#    linking the issue (its public URL, no token). Out, without
#    [intake.writeback]: finishing it runs nothing more.
git -C "$TF_ROOT" checkout -q -- openwarrant.toml
printf '\n[intake]\nfetch_argv = ["gh", "issue", "view", "{id}", "--json", "number,title,body,url"]\n' >> "$TF_ROOT/openwarrant.toml"
\cp -f "$TF_ROOT/openwarrant.toml" "$TF_TMP/read-only.toml"
: > "$TF_TMP/gh.log"
TF_OUT=$(tfj create --issue 12 -i "Fix it")
TF_G=$(tf_id "$TF_OUT"); TF_G1=$(tf_item "$TF_OUT" 0)
TF_LOG1=$(cat "$TF_TMP/gh.log")
TF_GM=$(cat "$TF_ROOT/docs/tickets/$TF_G/manifest.toml" 2>/dev/null)
TF_GI=$(cat "$TF_ROOT/docs/tickets/$TF_G/atoms/10-intent.md" 2>/dev/null)
tfw claim "$TF_G/$TF_G1" >/dev/null 2>&1
TF_DONE=$(tfj done "$TF_G/$TF_G1" --note "guarded"); TF_DS=$?
TF_LOG2=$(cat "$TF_TMP/gh.log")
TF_WB=$(tf_field "$TF_DONE" 'v["result"]["issue"]["writeback"]')
if [[ "$TF_LOG1" == "argv=issue view 12 --json number,title,body,url env=passed-through" && "$TF_LOG2" == "$TF_LOG1" \
    && $TF_DS -eq 0 && "$TF_WB" == off ]] \
    && tf_has 'issue = 12' "$TF_GM" && tf_has 'issue_url = "https://github.com/o/r/issues/12"' "$TF_GM" \
    && tf_has 'title = "Login fails on an empty password"' "$TF_GM" && tf_has 'Steps: submit the form empty.' "$TF_GI" \
    && ! tf_has 'secret' "$TF_GM"; then
    tf_ok "create --issue reads once; no write" "$TF_G from #12: one read (env passed through), the link recorded without the token; done ran nothing more (writeback off)"
else
    tf_fail "create --issue reads once; no write" "log1 '$TF_LOG1' log2 '$TF_LOG2'; done $TF_DS writeback '$TF_WB'; manifest: $TF_GM"
fi

# 7. With [intake.writeback]: finishing the last item runs exactly one
#    comment (its body naming what was done) and one close, with the
#    configured argv, in that order.
printf '\n[intake.writeback]\ncomment_argv = ["gh", "issue", "comment", "{id}", "--body", "{body}"]\nclose_argv = ["gh", "issue", "close", "{id}", "--reason", "completed"]\n' >> "$TF_ROOT/openwarrant.toml"
: > "$TF_TMP/gh.log"
TF_OUT=$(tfj create --issue 13 -i "Patch the guard" -i "Add the test")
TF_H=$(tf_id "$TF_OUT"); TF_H1=$(tf_item "$TF_OUT" 0); TF_H2=$(tf_item "$TF_OUT" 1)
tfw claim "$TF_H/$TF_H1" >/dev/null 2>&1; tfw done "$TF_H/$TF_H1" --note "null guarded" >/dev/null 2>&1
TF_MID=$(grep -c '^argv=' "$TF_TMP/gh.log")
tfw note "$TF_H" "root cause: an unchecked empty string" >/dev/null 2>&1
tfw claim "$TF_H/$TF_H2" >/dev/null 2>&1
TF_DONE=$(tfj done "$TF_H/$TF_H2" --note "regression test added"); TF_DS=$?
TF_CALLS=$(grep -c '^argv=' "$TF_TMP/gh.log")
TF_COMMENT=$(sed -n '/^argv=issue comment 13 --body /,/^argv=issue close/p' "$TF_TMP/gh.log")
TF_LAST=$(grep '^argv=' "$TF_TMP/gh.log" | tail -2 | cut -c1-30 | tr '\n' '|')
TF_CLOSE=$(grep '^argv=issue close' "$TF_TMP/gh.log")
TF_WB=$(tf_field "$TF_DONE" 'v["result"]["issue"]["writeback"]')
if [[ $TF_DS -eq 0 && "$TF_MID" == 1 && "$TF_CALLS" == 3 && "$TF_WB" == written \
    && "$TF_LAST" == "argv=issue comment 13 --body Do|argv=issue close 13 --reason co|" \
    && "$TF_CLOSE" == "argv=issue close 13 --reason completed env=passed-through" ]] \
    && tf_has "Done in ticket $TF_H: Login fails on an empty password" "$TF_COMMENT" \
    && tf_has "Patch the guard — done by claude" "$TF_COMMENT" && tf_has ": regression test added" "$TF_COMMENT" \
    && tf_has "root cause: an unchecked empty string" "$TF_COMMENT"; then
    tf_ok "writeback: one comment, one close" "$TF_H from #13: nothing written until its last item; then exactly comment (items, notes) and close, configured argv, in order"
else
    tf_fail "writeback: one comment, one close" "done $TF_DS ($TF_WB); calls mid $TF_MID / $TF_CALLS; last '$TF_LAST'; close '$TF_CLOSE'; comment: $(head -c 300 <<<"$TF_COMMENT")"
fi

# 8. A write that fails leaves the ticket done and reports the issue UNKNOWN,
#    by rule, the close never run; the journal says so.
: > "$TF_TMP/gh.log"; : > "$TF_TMP/gh.log.fail"
TF_OUT=$(tfj create --issue 14 -i "One thing")
TF_K=$(tf_id "$TF_OUT"); TF_K1=$(tf_item "$TF_OUT" 0)
tfw claim "$TF_K/$TF_K1" >/dev/null 2>&1
TF_ERR=$(tfw done "$TF_K/$TF_K1" 2>&1 >/dev/null); TF_S=$?
TF_J=$(tfj done "$TF_K/$TF_K1"); TF_SJ=$?
command rm -f "$TF_TMP/gh.log.fail"
TF_ST=$(tf_field "$(tfj tickets)" '[t["state"] for t in v["result"]["tickets"] if t["id"] == "'"$TF_K"'"][0]')
TF_CLOSED=$(grep -c '^argv=issue close' "$TF_TMP/gh.log")
TF_JL=$(grep '"ticket.issue_writeback"' "$TF_ROOT/docs/tickets/$TF_K/journal.jsonl")
if [[ $TF_S -eq 2 && "$TF_ST" == done && "$TF_CLOSED" == 0 && $TF_SJ -eq 0 ]] \
    && tf_has 'UNKNOWN (ticket.issue-unknown): GitHub issue #14 is UNKNOWN: comment unknown (gh failed' "$TF_ERR" \
    && tf_has 'HTTP 403: denied' "$TF_ERR" && tf_has "The ticket $TF_K is done and stays done" "$TF_ERR" \
    && tf_has '\"outcome\":\"unknown\"' "$TF_JL" && tf_has '\"step\":\"close\",\"outcome\":\"skipped\"' "$TF_JL" \
    && tf_has '"already_done":true' "$TF_J"; then
    tf_ok "a failed write reads UNKNOWN" "comment failed: exit 2, ticket.issue-unknown, $TF_K stays done, close not run, journalled unknown"
else
    tf_fail "a failed write reads UNKNOWN" "exit $TF_S state $TF_ST closes $TF_CLOSED: $TF_ERR; journal '$TF_JL'"
fi

# 9. A fetch_argv that writes is refused before any process starts; nothing
#    is created.
\cp -f "$TF_TMP/read-only.toml" "$TF_ROOT/openwarrant.toml"
sed -i 's|^fetch_argv = .*|fetch_argv = ["gh", "issue", "close", "{id}"]|' "$TF_ROOT/openwarrant.toml"
: > "$TF_TMP/gh.log"
TF_N0=$(find "$TF_ROOT/docs/tickets" -name manifest.toml | wc -l)
TF_ERR=$(tfw create --issue 15 2>&1 >/dev/null); TF_S=$?
TF_N1=$(find "$TF_ROOT/docs/tickets" -name manifest.toml | wc -l)
git -C "$TF_ROOT" checkout -q -- openwarrant.toml
if [[ $TF_S -eq 2 && ! -s "$TF_TMP/gh.log" && "$TF_N0" == "$TF_N1" ]] \
    && tf_has 'intake.fetch-not-a-read' "$TF_ERR" && tf_has 'Nothing was started' "$TF_ERR"; then
    tf_ok "a writing fetch_argv is refused" "fetch_argv naming close: intake.fetch-not-a-read, no process started, no ticket"
else
    tf_fail "a writing fetch_argv is refused" "exit $TF_S; gh log '$(cat "$TF_TMP/gh.log")'; tickets $TF_N0/$TF_N1; $TF_ERR"
fi

# 10. On THIS repository's tickets, the filters answer in well under a second
#     (asserted < 2000 ms, as plant 45 asserts for the loop). Read-only.
tf_ms() { local s e; s=$(date +%s%N); "$@" >/dev/null 2>&1; local rc=$?; e=$(date +%s%N); echo "$(( (e - s) / 1000000 )) $rc"; }
TF_TIMES=""; TF_SLOW=""
for args in "tickets --search ticket" "tickets --state done" "tickets --label none-such" "tickets --text kernel"; do
    # shellcheck disable=SC2086
    read -r ms rc < <(tf_ms env -u OPENWARRANT_ACTOR "$TF_WAR" --root "$REPO_ROOT" $args)
    TF_TIMES="$TF_TIMES ${args#tickets }=${ms}ms"
    [[ $rc -eq 0 && $ms -lt 2000 ]] || TF_SLOW="$TF_SLOW ${args}(${ms}ms,exit $rc)"
done
if [[ -z "$TF_SLOW" ]]; then
    tf_ok "filters are fast here" "$TF_TIMES"
else
    tf_fail "filters are fast here" "slow or failed:$TF_SLOW"
fi

command rm -rf "$TF_TMP"
corpus_gone "$TF_ROOT"
unset TF_ROOT TF_TMP TF_WAR TF_E TF_A TF_A1 TF_A2 TF_A3 TF_B TF_B1 TF_C TF_C1 TF_D TF_D1 TF_OUT TF_BAD \
    TF_ALL TF_ERR TF_ERR1 TF_ERR2 TF_ERR3 TF_S TF_S1 TF_S2 TF_S3 TF_SHOW0 TF_SHOW1 TF_P0 TF_P1 TF_READY \
    TF_LS TF_MODEL TF_OLD TF_MDIFF TF_CDIFF TF_CLINE TF_ST TF_EDIFF TF_N0 TF_N1 TF_ADIFF TF_GOOD TF_G \
    TF_G1 TF_LOG1 TF_LOG2 TF_GM TF_GI TF_DONE TF_DS TF_WB TF_H TF_H1 TF_H2 TF_MID TF_CALLS TF_COMMENT \
    TF_LAST TF_CLOSE TF_K TF_K1 TF_J TF_SJ TF_CLOSED TF_JL TF_TIMES TF_SLOW
