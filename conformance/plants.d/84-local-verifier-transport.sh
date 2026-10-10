# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Synthetic transport controls only: this gate never loads or calls a model.
if python3 -B "$REPO_ROOT/tools/verifier/test_local_verifier.py"; then
    printf 'ok    %-34s loopback, redirect, model/tool/truncation refusal and immutable input binding\n' "local verifier transport"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s synthetic transport control failed\n' "local verifier transport"
    FAILED=$((FAILED + 1))
fi
