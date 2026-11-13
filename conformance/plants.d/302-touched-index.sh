# shellcheck shell=bash
# An act on one ticket rebuilds that ticket's lines of the agent index and
# its parents', not every ticket's (docs/generated/llms.txt;
# context::refresh_index_touched).
#
# One scratch repository with a compiled index, an epic over an epic over a
# ticket, a directory ticket, and bystanders. Every claim is paired with the
# refusal that shows the control can say no:
# - after `add`, `note`, `start`, `promote` and `done` on the innermost
#   ticket, and `add` and `done` on a directory ticket, the index is what a
#   fresh compilation writes (`war check --generated` passes each time);
# - finishing the innermost ticket moves three lines, its own and both
#   epics' (open -> done), and no bystander's; reopening an item moves them
#   again (-> in_progress);
#   with one epic's stale line put back by hand, `context-index.drift`;
# - a bystander's line that is wrong before the act is not repaired by it:
#   only the touched ticket's blocks are rebuilt, and the check still says
#   `context-index.drift` until `war compile` rewrites the file;
# - `war plan edit --part-of`, which moves a ticket between parents, still
#   rebuilds every ticket's block: the wrong bystander line is gone after it.

echo "== one ticket's act rebuilds its lines of the index and its parents' =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID
TI_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$TI_T")
TI_ROOT=$(scratch_corpus TI)
[[ -d "${TI_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
TI_WAR="$REPO_ROOT/${WAR#./}"
TI_IDX="$TI_ROOT/docs/generated/llms.txt"
ti_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ti_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ti_war() {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=plant \
        "$TI_WAR" --root "$TI_ROOT" "$@" </dev/null
}
ti_create() {
    ti_war --json create "$@" 2>"$TI_T/err" \
        | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])'
}
ti_gen() {
    ti_war --json check --generated 2>/dev/null | python3 -c "
import json, sys
v = json.load(sys.stdin)
print(' '.join(sorted(d['severity'] + ':' + d['rule'] for d in v['diagnostics'] if d['rule'].startswith('context-index'))))"
}
ti_state() { sed -n "s/^- $1 · ticket · \([^ ]*\) · .*/\1/p" "$TI_IDX"; }
ti_line() { grep "^- $1 · ticket · " "$TI_IDX"; }
# ti_step <label> <war args...>: run the act; note a check that does not pass.
TI_BAD=""
ti_step() {
    local label="$1"; shift
    ti_war "$@" >"$TI_T/step.out" 2>&1 || TI_BAD+=" $label:exit"
    local gen; gen=$(ti_gen)
    [[ $gen == "pass:context-index.drift" ]] || TI_BAD+=" $label:$gen"
}

[[ -f "$TI_IDX" ]] || { printf 'PLANT SETUP FAILED: the scratch corpus compiled no llms.txt\n' >&2; exit 9; }
TI_TOP=$(ti_create "The programme")
TI_MID=$(ti_create "A part of it" --part-of "$TI_TOP")
TI_LEAF=$(ti_create "The work itself" --part-of "$TI_MID" --item "First thing")
TI_DIR=$(ti_create --legacy "A directory ticket" --item "Its one item")
TI_BY=$(ti_create "A bystander" --item "Untouched")
TI_OTHER=$(ti_create "Another epic")
[[ -n $TI_TOP && -n $TI_MID && -n $TI_LEAF && -n $TI_DIR && -n $TI_BY && -n $TI_OTHER ]] \
    || { printf 'PLANT SETUP FAILED: the index plant could not create its tickets\n' >&2; exit 9; }
[[ $(ti_gen) == "pass:context-index.drift" ]] \
    || { printf 'PLANT SETUP FAILED: the index drifts before any act (%s)\n' "$(ti_gen)" >&2; exit 9; }
TI_ITEM=$(sed -n 's/.*(\(i-[0-9a-f]*\)).*/\1/p' "$(tk "$TI_ROOT/docs/tickets/$TI_LEAF.md")" | head -1)

# --- each act leaves a fresh compilation ---------------------------------------------
ti_step add add "$TI_LEAF" "Second thing"
ti_step note note "$TI_LEAF" "A note for whoever is next"
ti_step start start "$TI_LEAF"
ti_step dir-add add "$TI_DIR" "Its second item"
ti_war claim "$TI_DIR" >/dev/null 2>&1
ti_step dir-done done "$TI_DIR"
TI_DIR_STATE=$(ti_state "$TI_DIR")
if [[ -z $TI_BAD && -n $TI_ITEM && $TI_DIR_STATE == done ]]; then
    ti_ok "each act leaves a fresh index" "add, note, start on a document; add, done on a directory ticket: check --generated passes after each"
else
    ti_fail "each act leaves a fresh index" "wrong:${TI_BAD:- none}; item '$TI_ITEM'; directory ticket shows '$TI_DIR_STATE'; $(head -c 160 "$TI_T/step.out")"
fi

# --- finishing the leaf moves its line and both epics', and no other ----------------------
TI_BAD=""
ti_line "$TI_BY" > "$TI_T/by-before.txt"
ti_line "$TI_MID" > "$TI_T/mid-open.txt"
TI_BEFORE="$(ti_state "$TI_LEAF") $(ti_state "$TI_MID") $(ti_state "$TI_TOP")"
while read -r TI_OPEN; do
    [[ -n $TI_OPEN ]] && ti_step "done-$TI_OPEN" done "$TI_LEAF/$TI_OPEN"
done < <(sed -n 's/^- \[ \] .*(\(i-[0-9a-f]*\)).*/\1/p' "$(tk "$TI_ROOT/docs/tickets/$TI_LEAF.md")")
TI_AFTER="$(ti_state "$TI_LEAF") $(ti_state "$TI_MID") $(ti_state "$TI_TOP")"
ti_line "$TI_BY" > "$TI_T/by-after.txt"
cp "$TI_IDX" "$TI_T/done.txt"
python3 - "$TI_IDX" "$TI_MID" "$TI_T/mid-open.txt" <<'PY'
import sys
path, mid, old = sys.argv[1:]
old = open(old, encoding="utf-8").read()
lines = open(path, encoding="utf-8").read().splitlines(keepends=True)
out = [old if l.startswith(f"- {mid} · ticket · ") else l for l in lines]
assert out != lines
open(path, "w", encoding="utf-8").write("".join(out))
PY
TI_STALE=$(ti_gen)
cp "$TI_T/done.txt" "$TI_IDX"
ti_step reopen done "$TI_LEAF/$TI_ITEM" --reopen --reason "found one more thing"
TI_REOPENED="$(ti_state "$TI_LEAF") $(ti_state "$TI_MID") $(ti_state "$TI_TOP")"
if [[ -z $TI_BAD && $TI_BEFORE == "open open open" && $TI_AFTER == "done done done" ]] \
    && cmp -s "$TI_T/by-before.txt" "$TI_T/by-after.txt" && [[ -s "$TI_T/by-after.txt" ]] \
    && [[ $TI_STALE == "error:context-index.drift" && $TI_REOPENED == "in_progress in_progress in_progress" ]]; then
    ti_ok "done moves the parents' lines" "leaf and both epics: open -> done -> in_progress on reopen; the bystander's line untouched; an epic's stale line: drift"
else
    ti_fail "done moves the parents' lines" "wrong:${TI_BAD:- none}; before '$TI_BEFORE', after '$TI_AFTER', reopened '$TI_REOPENED'; stale line: '$TI_STALE'; $(head -c 160 "$TI_T/step.out")"
fi

# --- only the touched ticket's blocks are rebuilt; a move between parents rebuilds all ----
sed -i "s/^\(- $TI_BY · ticket · [^ ]* · \)A bystander/\1A bystander, edited by hand/" "$TI_IDX"
TI_HAND=$(ti_line "$TI_BY")
ti_war note "$TI_LEAF" "Another note" >/dev/null 2>&1
TI_KEPT=$(ti_line "$TI_BY"); TI_KEPT_GEN=$(ti_gen)
ti_war edit "$TI_LEAF" --part-of "$TI_OTHER" >"$TI_T/edit.out" 2>&1; TI_EDIT_RC=$?
TI_FIXED=$(ti_line "$TI_BY"); TI_FIXED_GEN=$(ti_gen)
if [[ $TI_HAND == *"edited by hand"* && $TI_KEPT == "$TI_HAND" && $TI_KEPT_GEN == "error:context-index.drift" ]] \
    && [[ $TI_EDIT_RC -eq 0 && $TI_FIXED != *"edited by hand"* && $TI_FIXED_GEN == "pass:context-index.drift" ]]; then
    ti_ok "only the touched blocks are rebuilt" "a note leaves a bystander's wrong line (drift stays); edit --part-of rebuilds every ticket's block"
else
    ti_fail "only the touched blocks are rebuilt" "after note: '$TI_KEPT_GEN' ($TI_KEPT); edit exit $TI_EDIT_RC: '$TI_FIXED_GEN' ($TI_FIXED); $(head -c 160 "$TI_T/edit.out")"
fi
