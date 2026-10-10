# shellcheck shell=bash
# OW-WAR-0096: no owner activation or deployed isolation is claimed.
ROLE_TMP=$(mktemp -d)
if TMPDIR="$ROLE_TMP" python3 "$REPO_ROOT/conformance/fixtures/authority-consumers/control.py" "$REPO_ROOT" "$REPO_ROOT/${WAR#./}"; then
    printf 'ok    %-34s current grant; irrelevant legacy ignored; missing store refused\n' "protected role consumers"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s protected role consumer public control failed\n' "protected role consumers"
    FAILED=$((FAILED + 1))
fi
rm -rf -- "$ROLE_TMP"
unset ROLE_TMP
