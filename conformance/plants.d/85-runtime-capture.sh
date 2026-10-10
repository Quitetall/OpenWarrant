# shellcheck shell=bash
# OW-WAR-0149: real local capture/source-retention observations, synthetic receipts.
RC_TMP=$(mktemp -d)
if python3 "$REPO_ROOT/conformance/fixtures/runtime-capture/control.py" "$REPO_ROOT" "$REPO_ROOT/${WAR#./}" "$RC_TMP"; then
    printf 'ok    %-34s replay/source-loss; stale/colliding/altered records refused\n' "runtime source capture"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s public CLI source-retention control failed\n' "runtime source capture"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -rf -- "$RC_TMP"
unset RC_TMP
