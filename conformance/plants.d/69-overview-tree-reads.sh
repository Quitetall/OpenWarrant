# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
if OVERVIEW_READS_OUT=$(python3 conformance/controls/overview-tree-reads.py "$WAR" 2>&1); then
    printf 'ok    %-34s %s\n' 'overview reuses tree observations' "$OVERVIEW_READS_OUT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' 'overview reuses tree observations' "$OVERVIEW_READS_OUT"
    FAILED=$((FAILED + 1))
fi
unset OVERVIEW_READS_OUT
