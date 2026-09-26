# shellcheck shell=bash
# t-515e — a plant file that pipes into a quiet grep is not run.
#
# lib.sh sets `set -o pipefail`, under which `producer | grep -q X` reads false
# whenever the producer has more than one pipe buffer still to write when
# `grep -q` exits at its first match: the producer takes SIGPIPE (141). Over
# a captured `war check` whose matching lines came to 5.7 KB, the old form of
# a 58-invalidation check read false 1874 times in 2000; `line_has` never.
# `run_plant_files` (lib.sh) scans each file first (`plant_quiet_grep_pipes`).
#
# Accepted: a file whose greps read a captured text (`line_has`, a here-string,
# a process substitution), that pipes into a grep that reads all its input
# (-c, -v, -o), that uses `||`, or names the pattern in a comment, runs and
# its pass counts. Refused: five files, one per spelling (-q, -qF, an option
# before -q, `|&` into --quiet, a `|` ending one line and grep -q starting the
# next), are each FAILED by name and line and never run. And `line_has` itself:
# true over more than a pipe buffer of matching lines every time, false when
# the filter matches nothing, and a -v test reads only the filter's lines.
#
# This file spells grep as two strings joined, so the scan it tests does not
# refuse it.

echo "== plant files: no pipe into a quiet grep (t-515e) =="
QG_DIR=$(mktemp -d)
QG="gr""ep"
qg_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
qg_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
{
    printf '# a comment may name: war check | %s -q x\n' "$QG"
    printf 'QG_T=$(printf "a\\nERROR b\\n"); QG_N=$(%s -c . <<<"$QG_T" | wc -l)\n' "$QG"
    printf 'line_has -E "^ERROR" -F b <<<"$QG_T" && %s -q a <<<"$QG_T" && %s -qF b < <(printf "%%s\\n" "$QG_T")\n' "$QG" "$QG"
    printf 'QG_V=$(printf "%%s\\n" "$QG_T" | %s -v a | %s -o b); [[ -n "$QG_V" ]] || %s -q zz <<<"$QG_T"\n' "$QG" "$QG" "$QG"
    printf 'echo 50-clean >> "%s/log"; PASSED=$((PASSED + 1))\n' "$QG_DIR"
} > "$QG_DIR/50-clean.sh"
qg_refused() { # <name> <line 2>
    {
        printf 'echo %s >> "%s/log"; PASSED=$((PASSED + 1))\n' "$1" "$QG_DIR"
        printf '%s\n' "$2"
    } > "$QG_DIR/$1.sh"
}
qg_refused 51-q "printf 'x\\n' | $QG -q x && :"
qg_refused 52-qF "if war check 2>&1 | $QG -qF 'OW-WAR-0001'; then :; fi"
qg_refused 53-late "! $QG -E '^ERROR' <<<\"\$X\" | $QG -E -q 'ticket'"
qg_refused 54-amp "war next |& $QG --quiet 'war claim'"
{
    printf 'echo 55-split >> "%s/log"; PASSED=$((PASSED + 1))\n' "$QG_DIR"
    printf 'war next 2>/dev/null |\n'
    printf '    %s -q "war claim" && :\n' "$QG"
} > "$QG_DIR/55-split.sh"

# qg_run <file...>  ->  the inner run's output, then "TOTALS <passed> <failed>"
qg_run() {
    (
        PASSED=0
        FAILED=0
        run_plant_files "$@"
        echo "TOTALS $PASSED $FAILED"
    )
}

mapfile -t QG_FILES < <(plant_files_in "$QG_DIR")
: > "$QG_DIR/log"
QG_OUT=$(qg_run "${QG_FILES[@]}")
QG_LOG=$(tr '\n' ' ' < "$QG_DIR/log")
if [[ "$QG_LOG" == *"50-clean"* ]] && ! grep -q '^FAIL  50-clean' <<<"$QG_OUT" \
    && [[ -z "$(plant_quiet_grep_pipes "$QG_DIR/50-clean.sh")" ]]; then
    qg_ok "captured greps run" "50-clean.sh ran and passed; line_has, a here-string, <(…), -c/-v/-o pipes, ||, a comment"
else
    qg_fail "captured greps run" "ran: $QG_LOG; $(grep '^FAIL' <<<"$QG_OUT" | head -1)"
fi
QG_MISSED=""
for QG_CASE in 51-q:2 52-qF:2 53-late:2 54-amp:2 55-split:3; do
    QG_NAME=${QG_CASE%%:*}
    [[ "$QG_LOG" != *"$QG_NAME"* ]] \
        && grep -q "^FAIL  $QG_NAME.sh .*pipes into a quiet grep.*: ${QG_CASE#*:}: " <<<"$QG_OUT" \
        || QG_MISSED="$QG_MISSED $QG_NAME"
done
if [[ -z "$QG_MISSED" ]] && grep -q '^TOTALS 1 5$' <<<"$QG_OUT"; then
    qg_ok "a pipe into a quiet grep is refused" "-q, -qF, -E -q, |& --quiet, a split pipe: each FAILED at its line, none ran; totals 1 passed, 5 failed"
else
    qg_fail "a pipe into a quiet grep is refused" "missed:$QG_MISSED; ran: $QG_LOG; $(tail -1 <<<"$QG_OUT")"
fi

# line_has over more than one pipe buffer of matching lines: never false.
QG_BIG=$(for ((QG_I = 0; QG_I < 120; QG_I++)); do printf 'PASS evidence.admissible  W-%03d: software.repo.war-check@1.0.0 · receipt reseals\n' "$QG_I"; done)
QG_FALSE=0
for ((QG_I = 0; QG_I < 200; QG_I++)); do
    line_has -E '^PASS +evidence\.admissible' -F 'W-000:' <<<"$QG_BIG" || QG_FALSE=$((QG_FALSE + 1))
done
if [[ ${#QG_BIG} -gt 8192 && $QG_FALSE -eq 0 ]] \
    && ! line_has -E '^ERROR' -F 'W-000:' <<<"$QG_BIG" \
    && ! line_has -F 'W-001:' -v 'PASS' <<<"$QG_BIG" \
    && line_has -F 'W-001:' -v 'W-000' <<<"$QG_BIG"; then
    qg_ok "line_has reads a captured text" "200/200 true over ${#QG_BIG} bytes; an empty filter and a -v over the filter's lines are false"
else
    qg_fail "line_has reads a captured text" "$QG_FALSE of 200 false over ${#QG_BIG} bytes, or a refusal case read true"
fi
rm -rf "$QG_DIR"
unset QG_DIR QG QG_OUT QG_LOG QG_FILES QG_CASE QG_NAME QG_MISSED QG_BIG QG_FALSE QG_I
unset -f qg_ok qg_fail qg_run qg_refused
