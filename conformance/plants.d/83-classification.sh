# shellcheck shell=bash
# OW-WAR-0023: source labels survive detached capture; disclosure never inherits internal.
CLASS_TMP=$(mktemp -d)
if python3 "$REPO_ROOT/conformance/fixtures/dispatch-classification/control.py" "$REPO_ROOT" "$REPO_ROOT/${WAR#./}" "$CLASS_TMP"; then
    printf 'ok    %-34s offline declared label; missing, unlisted and wildcard refused\n' "Dispatch classification disclosure"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s public CLI classification control failed\n' "Dispatch classification disclosure"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -rf -- "$CLASS_TMP"
unset CLASS_TMP
