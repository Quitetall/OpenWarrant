# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
if READ_VERBS_OUT=$(python3 conformance/controls/read-verbs-tree-reads.py "$WAR" 2>&1); then
    printf 'ok    %-34s %s\n' 'read verbs reuse tree observations' "$READ_VERBS_OUT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' 'read verbs reuse tree observations' "$READ_VERBS_OUT"
    FAILED=$((FAILED + 1))
fi
unset READ_VERBS_OUT
