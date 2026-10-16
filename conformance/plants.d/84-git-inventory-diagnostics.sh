# shellcheck shell=bash
# t-62891: a failed source observation is UNKNOWN, with bounded cause details.
GI_TMP=$(mktemp -d)
if python3 "$REPO_ROOT/conformance/fixtures/verifier/git-inventory.py" "$REPO_ROOT" "$REPO_ROOT/${WAR#./}" "$GI_TMP"; then
    printf 'ok    %-34s actual Git refusal and restore; no verdict\n' "Git inventory diagnostics"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s public CLI inventory diagnostic control\n' "Git inventory diagnostics"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -rf -- "$GI_TMP"
unset GI_TMP
