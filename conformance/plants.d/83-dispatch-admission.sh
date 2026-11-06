# shellcheck shell=bash
# Exact current signed contract and required immutable holders, with explicit prototype fallback.
ADMISSION_TMP=$(mktemp -d)
if python3 "$REPO_ROOT/conformance/fixtures/dispatch-admission/control.py" "$REPO_ROOT" "$REPO_ROOT/${WAR#./}" "$ADMISSION_TMP"; then
    printf 'ok    %-34s stale signed subject and missing holder refuse without writes; prototype remains open\n' "Dispatch current admission"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s public CLI admission control failed\n' "Dispatch current admission"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -rf -- "$ADMISSION_TMP"
unset ADMISSION_TMP
