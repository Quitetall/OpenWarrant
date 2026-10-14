# SPDX-License-Identifier: Apache-2.0
# shellcheck shell=bash
# SAS section 76.4: JSON migration is one report, including the negative control.
echo "== legacy migration report envelope =="
if MIGRATION_JSON_OUT=$(timeout 75 python3 "$REPO_ROOT/conformance/migration/test_cli_report.py" --war "$WAR" 2>&1); then
    printf 'ok    %-34s %s\n' 'migration report envelope' 'success, promotion refusal, text compatibility and read-only verification'
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' 'migration report envelope' 'public CLI controls failed'
    printf '%s\n' "$MIGRATION_JSON_OUT" | tail -40
    FAILED=$((FAILED + 1))
fi
unset MIGRATION_JSON_OUT
