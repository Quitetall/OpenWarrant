# shellcheck shell=bash
# t-b9610: a correct packet digest does not prove required context is present.
# Remove a required fixture and recompute the exact canonical identity, then
# require ingestion refusal without writes and acceptance after exact restore.
VS_TMP=$(mktemp -d)
python3 "$REPO_ROOT/conformance/fixtures/verifier/required-sources.py" \
    "$REPO_ROOT" "$REPO_ROOT/${WAR#./}" "$VS_TMP"
VS_STATUS=$?
if [[ "$VS_STATUS" -eq 0 ]]; then
    printf 'ok    %-34s fixture/task/code/prior and contract-source controls; exact packet restored\n' "required review source closure"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s public CLI closure control failed (%s)\n' "required review source closure" "$VS_STATUS"
    FAILED=$((FAILED + 1))
fi
rm -rf -- "$VS_TMP"
unset VS_TMP VS_STATUS
