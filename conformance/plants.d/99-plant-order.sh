# shellcheck shell=bash
# t-dc28 — plant files share prefixes, and their order decides nothing.
#
# `plants.d/NN-*.sh` prefixes are a grouping; branches add files without
# coordinating, so two `57-` files is normal. `run_plant_files` (lib.sh) runs
# them in byte order and starts each from the restored tree. Four throwaway
# plant files, run by the same function in a subshell with its own tally:
# two share `57-`; one leaves a new file under docs/warrants/ that `restore`
# cannot undo; the last passes only if that leak is gone before it starts.
# Accepted: both `57-` files run, in name order. Refused: the leaking file is
# FAILED by name and its leak removed. Run in reverse order, the totals are
# the same — the order changed nothing.

echo "== plant files: prefixes group, order decides nothing (t-dc28) =="
PO_DIR=$(mktemp -d)
PO_LEAK="docs/warrants/.plant-order-leak-$$"
po_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
po_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
for n in 57-b 57-a; do
    printf 'echo %s >> "%s/log"; PASSED=$((PASSED + 1))\n' "$n" "$PO_DIR" > "$PO_DIR/$n.sh"
done
printf 'echo 58-leak >> "%s/log"; : > "$REPO_ROOT/%s"; PASSED=$((PASSED + 1))\n' "$PO_DIR" "$PO_LEAK" > "$PO_DIR/58-leak.sh"
printf 'echo 59-after >> "%s/log"; if [[ -e "$REPO_ROOT/%s" ]]; then FAILED=$((FAILED + 1)); else PASSED=$((PASSED + 1)); fi\n' "$PO_DIR" "$PO_LEAK" > "$PO_DIR/59-after.sh"

# po_run <file...>  ->  the inner run's output, then "TOTALS <passed> <failed>"
po_run() {
    (
        PASSED=0
        FAILED=0
        run_plant_files "$@"
        echo "TOTALS $PASSED $FAILED"
    )
}

mapfile -t PO_FILES < <(plant_files_in "$PO_DIR")
: > "$PO_DIR/log"
PO_OUT=$(po_run "${PO_FILES[@]}")
PO_LOG=$(tr '\n' ' ' < "$PO_DIR/log")
if [[ "$PO_LOG" == "57-a 57-b 58-leak 59-after " ]] && grep -q '^TOTALS 3 1$' <<<"$PO_OUT"; then
    po_ok "two files share a prefix" "57-a and 57-b both run, in byte order, then 58, 59"
else
    po_fail "two files share a prefix" "ran: $PO_LOG; $(tail -1 <<<"$PO_OUT")"
fi
if grep -qF "FAIL  58-leak.sh" <<<"$PO_OUT" && grep -qF "?? $PO_LEAK" <<<"$PO_OUT" \
    && [[ ! -e "$REPO_ROOT/$PO_LEAK" ]]; then
    po_ok "a file that leaks is named" "58-leak.sh FAILED for $PO_LEAK, removed before 59 ran"
else
    po_fail "a file that leaks is named" "$(grep -v '^TOTALS' <<<"$PO_OUT" | head -2 | tr '\n' '|') leak present: $([[ -e "$REPO_ROOT/$PO_LEAK" ]] && echo yes || echo no)"
fi

# Reverse order: the same verdicts, file by file.
PO_REV=()
for ((i = ${#PO_FILES[@]} - 1; i >= 0; i--)); do PO_REV+=("${PO_FILES[i]}"); done
: > "$PO_DIR/log"
PO_OUT2=$(po_run "${PO_REV[@]}")
if [[ "$(tail -1 <<<"$PO_OUT2")" == "$(tail -1 <<<"$PO_OUT")" ]] && grep -qF "FAIL  58-leak.sh" <<<"$PO_OUT2" \
    && [[ ! -e "$REPO_ROOT/$PO_LEAK" ]]; then
    po_ok "reversed, the same result" "$(tail -1 <<<"$PO_OUT2") either way"
else
    po_fail "reversed, the same result" "forward: $(tail -1 <<<"$PO_OUT"); reversed: $(tail -1 <<<"$PO_OUT2")"
fi
rm -f "$REPO_ROOT/$PO_LEAK"
rm -rf "$PO_DIR"
