# shellcheck shell=bash
# OW-WAR-0149: synthetic public SDK refusals, not native runtime qualification.
# The legacy CLI does not consume this candidate interface yet. Exercise the
# exported library boundary rather than reporting a CLI gate that does not exist.
RB_LOG=$(mktemp)
if timeout 300s cargo test --manifest-path "$REPO_ROOT/Cargo.toml" \
    -p openwarrant-compiler --test runtime_basis > "$RB_LOG" 2>&1; then
    printf 'ok    %-34s all stages; stale/ambiguous/invalid evidence refused\n' "runtime basis SDK"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s public SDK controls failed\n' "runtime basis SDK"
    tail -n 25 "$RB_LOG"
    FAILED=$((FAILED + 1))
fi
/usr/bin/rm -f -- "$RB_LOG"
unset RB_LOG
