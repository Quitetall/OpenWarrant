# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
if SUBMISSION_BINDINGS_OUT=$(python3 conformance/controls/submission-bindings.py "$WAR" 2>&1); then
    printf 'ok    %-34s %s\n' 'submission binds compiled dispatch' "$SUBMISSION_BINDINGS_OUT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' 'submission binds compiled dispatch' "$SUBMISSION_BINDINGS_OUT"
    FAILED=$((FAILED + 1))
fi
unset SUBMISSION_BINDINGS_OUT
