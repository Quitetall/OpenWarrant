# shellcheck shell=bash
# OW-WAR-0095 preparation only: synthetic controls cannot qualify participants.
STUDY_TMP=$(mktemp -d)
if TMPDIR="$STUDY_TMP" python3 -m unittest discover -s "$REPO_ROOT/conformance/study" -v; then
    printf 'ok    %-34s measured interruptions retained; fabricated/overwriting input refused\n' "study recorder preparation"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s study recorder controls failed\n' "study recorder preparation"
    FAILED=$((FAILED + 1))
fi
rm -rf -- "$STUDY_TMP"
unset STUDY_TMP
