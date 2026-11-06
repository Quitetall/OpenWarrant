# shellcheck shell=bash
# The 1.0 wizard (OW-WAR-0067): it reports the state and writes nothing until
# a human answers a prompt. A wizard that mutates on --check would be the
# worst possible defect in a script whose whole purpose is human consent.

WIZ=scripts/release-1.0-wizard.sh
WIZ_BEFORE=$(git status --porcelain | sort)
WIZ_OUT=$(bash "$WIZ" --check 2>&1)
WIZ_STATUS=$?
WIZ_AFTER=$(git status --porcelain | sort)
if [[ $WIZ_STATUS -eq 0 ]] && [[ "$WIZ_BEFORE" == "$WIZ_AFTER" ]] \
    && grep -q 'nothing was written' <<< "$WIZ_OUT" \
    && grep -q 'licence:' <<< "$WIZ_OUT"; then
    printf 'ok    %-34s state reported, tree untouched\n' "the wizard --check"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s, tree changed: %s\n' "the wizard --check" "$WIZ_STATUS" \
        "$(diff <(echo "$WIZ_BEFORE") <(echo "$WIZ_AFTER") | head -3)"
    FAILED=$((FAILED + 1))
fi

# Every step is reachable: the resume flag accepts each step number, and an
# unknown flag is refused rather than ignored.
for n in 1 4 8; do
    out=$(bash "$WIZ" --check --from "$n" 2>&1)
    if [[ $? -eq 0 ]] && grep -q 'nothing was written' <<< "$out"; then
        printf 'ok    %-34s --from %s preflights\n' "the wizard resumes" "$n"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s --from %s\n' "the wizard resumes" "$n"
        FAILED=$((FAILED + 1))
    fi
done
out=$(bash "$WIZ" --nonsense 2>&1)
if [[ $? -eq 2 ]] && grep -q 'unknown argument' <<< "$out"; then
    printf 'ok    %-34s rejected, not ignored\n' "an unknown wizard flag"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "an unknown wizard flag" "$(head -1 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
