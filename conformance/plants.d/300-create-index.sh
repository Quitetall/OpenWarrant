# shellcheck shell=bash
# `war create` adds its own block to the agent index and reads no other
# ticket for it (docs/generated/llms.txt; context::refresh_index_created), and
# sizes a new id by every ticket of either encoding.
#
# One scratch repository with a compiled index. Every claim is paired with
# the refusal that shows the control can say no:
# - after a create (a document, one with items and a summary, a directory
#   ticket, a child of an open epic) the index is what a fresh compilation
#   writes, `war check --generated` passes, and the index gained exactly that
#   ticket's block; with the block taken back out by hand, the same check
#   reports `context-index.drift`;
# - a child of a parent the index shows `done` moves a line that is not its
#   own (the parent's parent is no longer done; the parent keeps showing its
#   finished checklist), and the index is still a fresh compilation; with the
#   old line put back, `context-index.drift`;
# - an index that does not split into its blocks is not patched: the create
#   is accepted, says `context-index.refresh`, and the file is byte-identical;
# - a repository with no index gets none from a create;
# - ids start at four hex digits and are longer once the store holds more
#   tickets than four digits keep apart, counted over documents and directory
#   tickets alike; the first ids, made while it is small, are four.

echo "== war create: one block of the index, and ids sized by every ticket =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID
CI_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$CI_T")
CI_ROOT=$(scratch_corpus CI)
[[ -d "${CI_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
CI_WAR="$REPO_ROOT/${WAR#./}"
CI_IDX="$CI_ROOT/docs/generated/llms.txt"
ci_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ci_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ci_war() {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=plant \
        "$CI_WAR" --root "$CI_ROOT" "$@" </dev/null
}
# ci_create <args...>: the new ticket's id on stdout; stderr in $CI_T/err.
ci_create() {
    ci_war --json create "$@" 2>"$CI_T/err" \
        | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])'
}
# ci_gen: what `war check --generated` says of the index, e.g. `pass:context-index.drift`.
ci_gen() {
    ci_war --json check --generated 2>/dev/null | python3 -c "
import json, sys
v = json.load(sys.stdin)
print(' '.join(sorted(d['severity'] + ':' + d['rule'] for d in v['diagnostics'] if d['rule'].startswith('context-index'))))"
}
ci_blocks() { grep -c '^## ' "$CI_IDX"; }
ci_state() { sed -n "s/^- $1 · ticket · \([^ ]*\) · .*/\1/p" "$CI_IDX"; }

[[ -f "$CI_IDX" ]] || { printf 'PLANT SETUP FAILED: the scratch corpus compiled no llms.txt\n' >&2; exit 9; }
[[ $(ci_gen) == "pass:context-index.drift" ]] \
    || { printf 'PLANT SETUP FAILED: the compiled index drifts before any create (%s)\n' "$(ci_gen)" >&2; exit 9; }

# --- a create adds its block, and only that --------------------------------------
CI_N0=$(ci_blocks)
CI_A=$(ci_create "A plain document")
CI_B=$(ci_create "One with items" --summary "Says what the work is for." --item "First" --item "Second")
CI_C=$(ci_create --legacy "A directory ticket" --item "Still listed")
CI_EPIC=$(ci_create "An open epic")
CI_KID=$(ci_create "Its child" --part-of "$CI_EPIC")
CI_GEN=$(ci_gen)
CI_N1=$(ci_blocks)
cp "$CI_IDX" "$CI_T/after.txt"
# The refusal: the same index without the last ticket's block is drift.
python3 - "$CI_IDX" "$CI_KID" <<'PY'
import re, sys
path, kid = sys.argv[1:]
text = open(path, encoding="utf-8").read()
blocks = re.split(r"(?m)^(?=## )", text)
kept = [b for b in blocks if f"\n- {kid} · ticket · " not in b]
assert len(kept) == len(blocks) - 1, (len(kept), len(blocks))
open(path, "w", encoding="utf-8").write("".join(kept))
PY
CI_GEN_CUT=$(ci_gen)
cp "$CI_T/after.txt" "$CI_IDX"
if [[ -n $CI_A && -n $CI_B && -n $CI_C && -n $CI_KID && $CI_GEN == "pass:context-index.drift" ]] \
    && (( CI_N1 == CI_N0 + 5 )) \
    && grep -qF '> Says what the work is for.' "$CI_IDX" \
    && [[ $(ci_state "$CI_A") == open && $(ci_state "$CI_C") == open && $(ci_state "$CI_EPIC") == open ]] \
    && [[ $CI_GEN_CUT == "error:context-index.drift" ]]; then
    ci_ok "a create adds its own block" "5 creates, $CI_N0 -> $CI_N1 blocks, a fresh compilation each; without the last block: drift"
else
    ci_fail "a create adds its own block" "blocks $CI_N0 -> $CI_N1; check: '$CI_GEN'; block cut: '$CI_GEN_CUT'; $(head -c 200 "$CI_T/err")"
fi

# --- under a done parent, a line that is not the new ticket's moves -----------------
CI_TOP=$(ci_create "A finished programme")
CI_MID=$(ci_create "A finished part" --part-of "$CI_TOP" --item "The one thing")
ci_war claim "$CI_MID" >/dev/null 2>&1
ci_war done "$CI_MID" >/dev/null 2>&1
CI_TOP_BEFORE=$(ci_state "$CI_TOP"); CI_MID_BEFORE=$(ci_state "$CI_MID")
grep "^- $CI_TOP · ticket · " "$CI_IDX" > "$CI_T/top-line.txt"
CI_LATE=$(ci_create "Found late" --part-of "$CI_MID")
CI_GEN=$(ci_gen)
CI_TOP_AFTER=$(ci_state "$CI_TOP"); CI_MID_AFTER=$(ci_state "$CI_MID")
cp "$CI_IDX" "$CI_T/after.txt"
python3 - "$CI_IDX" "$CI_TOP" "$CI_T/top-line.txt" <<'PY'
import sys
path, mid, old = sys.argv[1:]
old = open(old, encoding="utf-8").read()
lines = open(path, encoding="utf-8").read().splitlines(keepends=True)
out = [old if l.startswith(f"- {mid} · ticket · ") else l for l in lines]
assert out != lines
open(path, "w", encoding="utf-8").write("".join(out))
PY
CI_GEN_OLD=$(ci_gen)
cp "$CI_T/after.txt" "$CI_IDX"
if [[ $CI_MID_BEFORE == done && $CI_TOP_BEFORE == done && -n $CI_LATE ]] \
    && [[ $CI_MID_AFTER == done && $CI_TOP_AFTER == in_progress && $(ci_state "$CI_LATE") == open ]] \
    && [[ $CI_GEN == "pass:context-index.drift" && $CI_GEN_OLD == "error:context-index.drift" ]]; then
    ci_ok "a done parent's lines move" "the parent's parent: done -> in_progress, a fresh compilation; its old line put back: drift"
else
    ci_fail "a done parent's lines move" "parent $CI_MID_BEFORE -> $CI_MID_AFTER, its parent $CI_TOP_BEFORE -> $CI_TOP_AFTER; check: '$CI_GEN'; old line: '$CI_GEN_OLD'"
fi

# --- an index that does not split is not patched -------------------------------------
cp "$CI_IDX" "$CI_T/good.txt"
python3 - "$CI_IDX" <<'PY'
import re, sys
path = sys.argv[1]
blocks = re.split(r"(?m)^(?=## )", open(path, encoding="utf-8").read())
# The last two blocks swapped: every byte kept, the sources out of order.
blocks[-1], blocks[-2] = blocks[-2], blocks[-1]
open(path, "w", encoding="utf-8").write("".join(blocks))
PY
CI_BAD_SUM=$(sha256sum < "$CI_IDX")
CI_D=$(ci_create "Made beside a broken index"); CI_D_RC=$?
CI_BAD_AFTER=$(sha256sum < "$CI_IDX")
if [[ -n $CI_D && $CI_D_RC -eq 0 && -f "$(tk "$CI_ROOT/docs/tickets/$CI_D.md")" ]] \
    && grep -q 'context-index.refresh' "$CI_T/err" && [[ $CI_BAD_SUM == "$CI_BAD_AFTER" ]]; then
    ci_ok "a broken index is not patched" "the create is accepted and says context-index.refresh; the file is byte-identical"
else
    ci_fail "a broken index is not patched" "exit $CI_D_RC, id '$CI_D'; $(head -c 200 "$CI_T/err")"
fi

# --- no index, none made --------------------------------------------------------------
rm -f "$CI_IDX"
CI_E=$(ci_create "Made with no index")
if [[ -n $CI_E && ! -e "$CI_IDX" ]]; then
    ci_ok "no index, none made" "a create in a repository that compiled none writes none"
else
    ci_fail "no index, none made" "id '$CI_E'; llms.txt $([[ -e "$CI_IDX" ]] && echo exists || echo absent)"
fi

# --- ids are sized by every ticket ----------------------------------------------------
# Four hex digits keep 36 tickets apart at the 1% bound (core `ticket_id_len`);
# this store holds documents and directory tickets, and none of the first ten
# ids, nor any made here, may be shorter than four.
CI_FIRST_LEN=$(( ${#CI_A} - 2 ))
for i in $(seq 1 30); do
    if (( i % 5 == 0 )); then ci_create --legacy "Filler $i" >/dev/null; else ci_create "Filler $i" >/dev/null; fi
done
CI_COUNT=$({ ls "$CI_ROOT/docs/tickets" 2>/dev/null; ls "$CI_ROOT/docs/work" 2>/dev/null; } | sed 's/\.md$//' | sort -u | grep -c '^[tw]-')
CI_LONG=$(ci_create "Made in a store past the four-digit bound")
CI_LONG_LEN=$(( ${#CI_LONG} - 2 ))
if (( CI_FIRST_LEN == 4 && CI_COUNT > 37 && CI_LONG_LEN >= 5 )); then
    ci_ok "ids lengthen with the store" "the first id has 4 hex digits; the one after $((CI_COUNT - 1)) tickets has $CI_LONG_LEN"
else
    ci_fail "ids lengthen with the store" "first $CI_FIRST_LEN; after $((CI_COUNT - 1)) tickets: $CI_LONG_LEN ($CI_LONG)"
fi
