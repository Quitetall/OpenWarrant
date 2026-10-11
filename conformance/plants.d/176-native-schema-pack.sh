# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# OW149: portable native draft schemas remain separate from active v1 identity.
# Mutations are only in a disposable fixture; no source or signed record moves.
NSP_TMP=$(mktemp -d)
if battery_tool "$NSP_TMP" war -p openwarrant-cli --features schema; then
    NSP_OUT=$(python3 "$REPO_ROOT/conformance/native_schema_pack.py" \
        "$REPO_ROOT/schemas/native-draft" "$NSP_TMP/war" 2>&1)
    NSP_CODE=$?
else
    NSP_OUT="schema-featured fixture binary did not build"
    NSP_CODE=125
fi
if [[ $NSP_CODE -eq 0 ]]; then
    printf 'ok    %-34s %s\n' "portable native schemas and controls" "$NSP_OUT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s: %s\n' "portable native schemas and controls" "$NSP_CODE" "$NSP_OUT"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -rf -- "$NSP_TMP"
unset NSP_TMP NSP_OUT NSP_CODE
