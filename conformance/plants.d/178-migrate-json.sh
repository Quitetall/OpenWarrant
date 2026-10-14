# SPDX-License-Identifier: Apache-2.0
# shellcheck shell=bash
# SAS section 76.4: JSON migration is one report, including the negative control.
echo "== legacy migration report envelope =="
if timeout 75 python3 "$REPO_ROOT/conformance/migration/test_cli_report.py" --war "$WAR" >"$TMP/migration-report-controls.log" 2>&1; then
    printf 'ok    %-34s %s\n' 'migration report envelope' 'success, promotion refusal, text compatibility and read-only verification'
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' 'migration report envelope' 'public CLI controls failed'
    tail -40 "$TMP/migration-report-controls.log"
    FAILED=$((FAILED + 1))
fi
