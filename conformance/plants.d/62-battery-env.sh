# shellcheck shell=bash
# t-354e — a battery gives the same answer whatever its caller exports.
#
# Run with CLAUDE_PERFORMER_MODEL exported, 62-verifier's 'independence only
# where true' failed on every battery: its "performer model unset" case read
# the caller's model, and the wrapper then claimed a distinct model. lib.sh now
# unsets what the wrappers and `war` read (its own comment names each), and a
# plant that needs one sets it on the command it runs.
#
# A representative subset — 62-verifier (the CLAUDE_* reader) and 45-tickets
# (OPENWARRANT_ACTOR) — run in a child battery three times, each with its own
# tally:
#   accepting: the totals with the caller's CLAUDE_*/OPENWARRANT_ACTOR/VISUAL
#              exported equal the totals with none of them set, and
#              'independence only where true' passes both ways
#   refusing:  the same variables exported AFTER lib.sh (what a battery
#              without the unset saw) make 62-verifier fail — so the equality
#              above is the unset at work, not a plant that reads nothing

echo "== the battery ignores the caller's environment (t-354e) =="
BE_TMP=$(mktemp -d)
be_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
be_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
BE_LEAK=(CLAUDE_PERFORMER_MODEL=claude-opus-5-5 CLAUDE_VERIFIER_MODEL=claude-opus-5-5
    "CLAUDE_VERIFIER_LOG=$BE_TMP/vlog" CLAUDE_BIN=/bin/false
    OPENWARRANT_ACTOR=someone-else VISUAL=/bin/false EDITOR=/bin/false)
BE_UNSET=(-u CLAUDE_PERFORMER_MODEL -u CLAUDE_VERIFIER_MODEL -u CLAUDE_VERIFIER_LOG -u CLAUDE_BIN
    -u OPENWARRANT_ACTOR -u VISUAL -u EDITOR)

# be_run <after-lib.sh> <env args...>  ->  "TOTALS <passed> <failed>"; the
# child's plant lines are kept in $BE_TMP/last.log
be_run() {
    local after=$1
    shift
    (cd "$REPO_ROOT" && env "$@" BE_AFTER="$after" bash -c '
        source conformance/lib.sh >/dev/null
        eval "$BE_AFTER"
        run_plant_files conformance/plants.d/45-tickets.sh conformance/plants.d/62-verifier.sh
        echo "TOTALS $PASSED $FAILED"
    ' 2>&1) >"$BE_TMP/last.log"
    grep -E '^TOTALS ' "$BE_TMP/last.log" | tail -1
}

BE_CLEAN=$(be_run ':' "${BE_UNSET[@]}")
BE_CLEAN_LOG=$(grep -E '^(ok|FAIL) ' "$BE_TMP/last.log")
BE_EXPORTED=$(be_run ':' "${BE_LEAK[@]}")
BE_EXPORTED_LOG=$(grep -E '^(ok|FAIL) ' "$BE_TMP/last.log")
# Totals, and 62-verifier's line: the child also runs lib.sh's positive corpus
# check, whose verdict is this repository's and the same both ways.
if [[ -n "$BE_CLEAN" && "$BE_CLEAN" == "$BE_EXPORTED" ]] \
    && grep -q '^ok    independence only where true' <<<"$BE_CLEAN_LOG" \
    && grep -q '^ok    independence only where true' <<<"$BE_EXPORTED_LOG" \
    && [[ ! -e "$BE_TMP/vlog" ]]; then
    be_ok "exported or not, the same totals" "${BE_CLEAN#TOTALS } (passed failed) both ways; no verifier log written"
else
    be_fail "exported or not, the same totals" "unset: ${BE_CLEAN:-none}; exported: ${BE_EXPORTED:-none}; $(diff <(echo "$BE_CLEAN_LOG") <(echo "$BE_EXPORTED_LOG") | grep -E '^[<>]' | head -2 | tr '\n' '|')"
fi

BE_LEAKED=$(be_run 'export CLAUDE_PERFORMER_MODEL=claude-opus-5-5' "${BE_UNSET[@]}")
if [[ -n "$BE_LEAKED" && "$BE_LEAKED" != "$BE_CLEAN" ]] \
    && grep -q '^FAIL  independence only where true' "$BE_TMP/last.log"; then
    be_ok "the model re-exported after lib.sh" "62-verifier fails 'independence only where true' (${BE_LEAKED#TOTALS })"
else
    be_fail "the model re-exported after lib.sh" "wanted 'independence only where true' to fail: ${BE_LEAKED:-no totals}"
fi

command rm -rf "$BE_TMP"
