# shellcheck shell=bash
# t-0e4f — a plant file that names a fixed path under /tmp is not run.
#
# Batteries run side by side, one per Warrant, each in its own clone; a fixed
# /tmp path is what they still share, and plant 64's correction response was
# overwritten that way between its write and its ingest. `run_plant_files`
# (lib.sh) scans each file first (`plant_fixed_tmp_paths`). Two throwaway
# plant files, run by that function in a subshell with its own tally.
# Accepted: a file whose scratch comes from mktemp, a mktemp template, the
# temp ROOT (`${TMPDIR:-/tmp}`), a tmp/ inside another path, and a fixed path
# in a comment runs, and its pass counts. Refused: a file that writes one
# fixed path under /tmp is FAILED by name and line, and never runs.
#
# This file spells the root as two strings joined, so the scan it tests does
# not refuse it.

echo "== plant files: no fixed /tmp path (t-0e4f) =="
FT_DIR=$(mktemp -d)
FT_ROOT="/tm""p"
ft_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ft_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
{
    printf '# a comment may name %s/openwarrant-anything.toml\n' "$FT_ROOT"
    printf 'FT_A=$(mktemp -d); FT_B=$(mktemp %s/war-plant.XXXXXX)\n' "$FT_ROOT"
    printf 'FT_C="${TMPDIR:-%s}/war-plant-$$"; FT_D="$FT_A/tmp/inner"\n' "$FT_ROOT"
    printf '%s\n' 'rm -rf "$FT_A" "$FT_B"; echo 50-clean >> "$FT_DIR/log"; PASSED=$((PASSED + 1))'
} > "$FT_DIR/50-clean.sh"
{
    printf '%s\n' 'echo 51-fixed >> "$FT_DIR/log"; PASSED=$((PASSED + 1))'
    printf 'RESP=%s/openwarrant-plant-fixed.toml\n' "$FT_ROOT"
} > "$FT_DIR/51-fixed.sh"

# ft_run <file...>  ->  the inner run's output, then "TOTALS <passed> <failed>"
ft_run() {
    (
        PASSED=0
        FAILED=0
        run_plant_files "$@"
        echo "TOTALS $PASSED $FAILED"
    )
}

mapfile -t FT_FILES < <(plant_files_in "$FT_DIR")
: > "$FT_DIR/log"
FT_OUT=$(ft_run "${FT_FILES[@]}")
FT_LOG=$(tr '\n' ' ' < "$FT_DIR/log")
if [[ "$FT_LOG" == *"50-clean"* ]] && ! grep -q '^FAIL  50-clean' <<<"$FT_OUT" \
    && [[ -z "$(plant_fixed_tmp_paths "$FT_DIR/50-clean.sh")" ]]; then
    ft_ok "mktemp scratch and the temp root run" "50-clean.sh ran and passed; a template, \${TMPDIR:-…}, a nested tmp/, a comment"
else
    ft_fail "mktemp scratch and the temp root run" "ran: $FT_LOG; $(grep '^FAIL' <<<"$FT_OUT" | head -1)"
fi
if [[ "$FT_LOG" != *"51-fixed"* ]] && grep -q '^FAIL  51-fixed.sh .*fixed /tmp path.*2: RESP=' <<<"$FT_OUT" \
    && grep -q '^TOTALS 1 1$' <<<"$FT_OUT"; then
    ft_ok "a fixed /tmp path is refused" "51-fixed.sh FAILED at line 2 and never ran; totals 1 passed, 1 failed"
else
    ft_fail "a fixed /tmp path is refused" "ran: $FT_LOG; $(grep -v '^TOTALS' <<<"$FT_OUT" | head -2 | tr '\n' '|') $(tail -1 <<<"$FT_OUT")"
fi
rm -rf "$FT_DIR"
unset FT_DIR FT_ROOT FT_OUT FT_LOG FT_FILES
unset -f ft_ok ft_fail ft_run
