# shellcheck shell=bash
# OW-WAR-0148 M1 / t-3a90c: one pre-bound execution for compatible selected
# Warrants; synthetic fixtures establish no real authority or assurance.
echo '== shared preparation observations =='
SGP_WAR="$REPO_ROOT/${WAR#./}"
for SGP_CASE in shared mutate-input mutate-contract mutate-gate warrant-specific fail-gate different-orders earlier-failure timeout-second; do
  for SGP_JOBS in 1 2; do
    if [[ "$SGP_JOBS" == 2 && "$SGP_CASE" != different-orders && "$SGP_CASE" != earlier-failure ]]; then
        continue
    fi
    SGP_ARGS=()
    [[ "$SGP_CASE" == shared ]] || SGP_ARGS+=("--$SGP_CASE")
    SGP_LOG=$(mktemp)
    if python3 "$REPO_ROOT/conformance/shared_gate_prepare.py" \
        --war "$SGP_WAR" --jobs "$SGP_JOBS" "${SGP_ARGS[@]}" >"$SGP_LOG" 2>&1; then
        printf 'ok    %-34s %s, jobs=%s\n' 'shared preparation' "$SGP_CASE" "$SGP_JOBS"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s, jobs=%s\n' 'shared preparation' "$SGP_CASE" "$SGP_JOBS"
        tail -12 "$SGP_LOG"
        FAILED=$((FAILED + 1))
    fi
    /usr/bin/rm -f "$SGP_LOG"
  done
done
unset SGP_WAR SGP_CASE SGP_ARGS SGP_LOG SGP_JOBS
