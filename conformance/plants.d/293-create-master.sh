# shellcheck shell=bash
# New work is a master (R5, docs/design/one-format.md "Where new work goes"):
# `war create` writes `docs/work/<w-id>/<id>-<slug>.md` with its state beside
# it, and every daily command works it as it works a `t-` ticket. Each claim
# is paired with its refusal.
#
# Accepted: create writes one document under docs/work/ and touches no view;
# start, note, a question and its listing, and `war done` under `[tickets]
# done_test` (ticked at observed) all run on the `w-` id; `war status <id>`
# and `war check <id>` read it as a ticket; a renamed slug changes no id; a
# `t-` ticket written before stays where it is and is worked in place; `war
# plan new` takes its id on the same path (docs/work/, the clone's register).
# Refused: `done` with a failing default test ticks nothing; nothing is
# written under docs/tickets/ by create; a second document claiming the id is
# `format.document-duplicate`, and the ticket is not worked.

plant_layout work "what war create writes in docs/work/, and that a t- ticket beside it is worked in place"
echo "== one layout: war create writes a master =="
CM_WAR="$REPO_ROOT/${WAR#./}"
cm_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cm_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
CM_T=$(mktemp -d)
CM_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_VERBOSE -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID
    -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1)
cm_g() { "${CM_ENV[@]}" git "$@" >/dev/null 2>&1; }
cm_as() {
    local actor="$1"; shift
    (cd "$CM_R" && "${CM_ENV[@]}" OPENWARRANT_ACTOR="$actor" timeout 120 "$CM_WAR" "$@" \
        >"$CM_T/out" 2>"$CM_T/err" </dev/null)
}
cm_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["id"])' "$CM_T/out"; }
CM_R="$CM_T/r"

mkdir -p "$CM_R/docs/tickets"
cm_g -C "$CM_R" init -q -b main
cm_as owner init --done-test 'test -f passes'
# A ticket from before docs/work/: one document under docs/tickets/.
cat >"$CM_R/docs/tickets/t-0a1b.md" <<'DOC'
---
format: openwarrant/1.0
profile: ticket
alias: t-0a1b
uuid: 01a1d2f8-0000-7000-8000-0000000000a1
priority: 2
created_at: "2026-11-01T00:00:00Z"
created_by: "owner"
summary: "An older ticket"
---
# An older ticket

## Checklist

- [ ] keep working (i-0a01)
DOC
cm_g -C "$CM_R" add -A
cm_g -C "$CM_R" commit -qm seed
if ! grep -q 'done_test' "$CM_R/openwarrant.toml"; then
    printf 'PLANT SETUP FAILED: init --done-test: %s\n' "$(head -c 300 "$CM_T/err")" >&2; exit 9
fi

# --- create writes one master, and no view ------------------------------------------
cm_as alice --json create "Ship the audit log" --item "write it"; CM_RC=$?; CM_ID=$(cm_id)
CM_DOC="docs/work/$CM_ID/$CM_ID-ship-the-audit-log.md"
CM_NEW=$(cd "$CM_R" && "${CM_ENV[@]}" git status --short --untracked-files=all | grep -v '^?? \.openwarrant/' | sed 's/^?? //' | sort | tr '\n' ' ')
if [[ $CM_RC == 0 && "$CM_ID" == w-* && -f "$CM_R/$CM_DOC" ]] \
    && [[ "$CM_NEW" == "docs/work/$CM_ID/journal.jsonl $CM_DOC " ]] \
    && [[ ! -e "$CM_R/docs/tickets/$CM_ID.md" && ! -e "$CM_R/docs/ROADMAP.md" && ! -e "$CM_R/docs/pages" ]]; then
    cm_ok "create writes a master" "docs/work/<w-id>/<id>-<slug>.md and its journal; nothing under docs/tickets/, no view"
else
    cm_fail "create writes a master" "exit $CM_RC id '$CM_ID' new: $CM_NEW | $(head -c 200 "$CM_T/err")"
fi

# --- the loop on a w- id: start, note, question ---------------------------------------
cm_as alice start "$CM_ID"; CM_S=$?
cm_as alice plan note "$CM_ID" "halfway"; CM_N=$?
CM_ITEM=$(grep -o 'i-[0-9a-f]*' "$CM_R/$CM_DOC" | head -1)
cm_as alice ask "$CM_ID" "$CM_ITEM" "Which table holds it?"; CM_Q=$?
cm_as alice questions; CM_QL=$(grep -c "$CM_ID Q-001  Which table holds it" "$CM_T/out")
cm_as bob plan note "$CM_ID" --answer Q-001 "the audit table"; CM_A=$?
cm_as alice status "$CM_ID"; CM_SHOWN=$(grep -c "^# $CM_ID — Ship the audit log" "$CM_T/out")
cm_as alice --json check "$CM_ID"; CM_CHK=$(grep -c '"ticket.well-formed"' "$CM_T/out")
if [[ $CM_S == 0 && $CM_N == 0 && $CM_Q == 0 && $CM_QL == 1 && $CM_A == 0 && $CM_SHOWN == 1 && $CM_CHK -ge 1 ]]; then
    cm_ok "the loop runs on a w- id" "start, note, a question listed and answered; status and check read it as a ticket"
else
    cm_fail "the loop runs on a w- id" "start $CM_S note $CM_N ask $CM_Q listed $CM_QL answered $CM_A shown $CM_SHOWN check $CM_CHK: $(head -c 300 "$CM_T/err")"
fi

# --- done under done_test: refused while it fails, observed once it passes -----------
cm_as alice done "$CM_ID"; CM_FAIL=$?; CM_TICKED=$(grep -c '^- \[x\]' "$CM_R/$CM_DOC")
touch "$CM_R/passes"
cm_as alice done "$CM_ID" --note "written"; CM_PASS=$?; cat "$CM_T/out" "$CM_T/err" >"$CM_T/done"
if [[ $CM_FAIL != 0 && $CM_TICKED == 0 && $CM_PASS == 0 ]] && grep -q 'observed' "$CM_T/done" \
    && grep -q '^- \[x\] write it' "$CM_R/$CM_DOC"; then
    cm_ok "done runs the default test" "refused and nothing ticked while it fails; ticked at observed once it passes"
else
    cm_fail "done runs the default test" "failing $CM_FAIL (ticked $CM_TICKED), passing $CM_PASS: $(head -c 300 "$CM_T/done")"
fi

# --- a t- ticket stays where it is, and is worked there --------------------------------
cm_as bob start t-0a1b; CM_S=$?
cm_as bob done t-0a1b; CM_D=$?
if [[ $CM_S == 0 && $CM_D == 0 && ! -e "$CM_R/docs/work/t-0a1b" ]] && grep -q '^- \[x\] keep working' "$CM_R/docs/tickets/t-0a1b.md"; then
    cm_ok "a t- ticket is worked in place" "started and finished in docs/tickets/; nothing moved by itself"
else
    cm_fail "a t- ticket is worked in place" "start $CM_S done $CM_D: $(head -c 300 "$CM_T/err")"
fi

# --- the slug is cosmetic; a second document under the id is refused -----------------
cm_as alice --json create "Rename my slug" --item "one"; CM_ID2=$(cm_id)
mv "$CM_R/docs/work/$CM_ID2/$CM_ID2-rename-my-slug.md" "$CM_R/docs/work/$CM_ID2/$CM_ID2-another-name.md"
cm_as alice start "$CM_ID2"; CM_S=$?
cp "$CM_R/docs/work/$CM_ID2/$CM_ID2-another-name.md" "$CM_R/docs/work/$CM_ID2/$CM_ID2-a-twin.md"
cm_as alice plan note "$CM_ID2" "to which one?"; CM_TWIN=$?; cat "$CM_T/out" "$CM_T/err" >"$CM_T/twin"
rm -f "$CM_R/docs/work/$CM_ID2/$CM_ID2-a-twin.md"
if [[ $CM_S == 0 && $CM_TWIN != 0 ]] && grep -q 'format.document-duplicate\|ticket.unknown' "$CM_T/twin"; then
    cm_ok "the slug is cosmetic" "renamed, the id still starts; two documents under one id are not worked"
else
    cm_fail "the slug is cosmetic" "start after rename $CM_S, twin $CM_TWIN: $(head -c 300 "$CM_T/twin")"
fi

# --- war plan new takes its id on the same path ----------------------------------------
cm_as alice --json plan new "A formal order"; CM_RC=$?
CM_ORDER=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["result"]["alias"])' "$CM_T/out" 2>/dev/null)
if [[ $CM_RC == 0 && "$CM_ORDER" == w-* && -f "$CM_R/docs/work/$CM_ORDER/$CM_ORDER-a-formal-order.md" \
    && -f "$CM_R/.git/openwarrant/tickets/$CM_ORDER" ]]; then
    cm_ok "plan new takes the same path" "a w- id under docs/work/, held in the clone's register as a ticket's is"
else
    cm_fail "plan new takes the same path" "exit $CM_RC id '$CM_ORDER': $(head -c 300 "$CM_T/err")"
fi

[[ -n "$CM_T" && "$CM_T" == */tmp.* ]] && /usr/bin/rm -rf "$CM_T"
