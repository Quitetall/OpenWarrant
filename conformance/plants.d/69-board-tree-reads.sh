# shellcheck shell=bash
# t-84d84: the board remains a complete reader within the HTTP deadline.
if BOARD_READS_OUT=$(python3 conformance/controls/board-tree-reads.py "$WAR" 2>&1); then
    printf 'ok    %-34s %s\n' 'board reuses tree observations' "$BOARD_READS_OUT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' 'board reuses tree observations' "$BOARD_READS_OUT"
    FAILED=$((FAILED + 1))
fi
unset BOARD_READS_OUT
