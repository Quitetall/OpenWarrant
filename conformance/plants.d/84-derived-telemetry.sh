# shellcheck shell=bash
# t-ccf4e / OW-WAR-0041: exact opt-in metrics, never fabricated observations.
DM_TMP=$(mktemp -d)
if python3 "$REPO_ROOT/conformance/fixtures/telemetry/derived.py" "$REPO_ROOT" "$REPO_ROOT/${WAR#./}" "$DM_TMP"; then
    printf 'ok    %-34s exact snapshots and ratios; missing/colliding/linked controls refused\n' "derived telemetry"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s public CLI derived telemetry control failed\n' "derived telemetry"
    FAILED=$((FAILED + 1))
fi
rm -rf -- "$DM_TMP"
unset DM_TMP
