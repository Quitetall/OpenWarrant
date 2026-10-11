# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# OW90: exact committed review subjects, incomplete evidence and refused ambiguity.
INVR_TMP=$(mktemp -d)
if INVR_OUT=$(TMPDIR="$INVR_TMP" timeout 180 python3 -m unittest discover \
    -s "$REPO_ROOT/conformance/release" -p test_review_inventory.py 2>&1); then
    printf 'ok    %-34s %s\n' "release review inventory controls" "$INVR_OUT"
    PASSED=$((PASSED + 1))
else
    INVR_CODE=$?
    printf 'FAIL  %-34s exit %s: %s\n' "release review inventory controls" "$INVR_CODE" "$INVR_OUT"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -r -- "$INVR_TMP"
unset INVR_TMP INVR_OUT INVR_CODE
