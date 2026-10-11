# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Exact worktree checkpoints supply context, never authority or retention policy.
SW_OUT=$(python3 "$REPO_ROOT/conformance/source_workspace.py" "$REPO_ROOT" "$REPO_ROOT/${WAR#./}" 2>&1)
SW_CODE=$?
if [[ $SW_CODE -eq 0 ]]; then
    printf 'ok    %-34s %s\n' "source workspace controls" "$SW_OUT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s: %s\n' "source workspace controls" "$SW_CODE" "$SW_OUT"
    FAILED=$((FAILED + 1))
fi
unset SW_OUT SW_CODE
