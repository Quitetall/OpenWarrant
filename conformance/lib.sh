#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Planted-violation battery (SAS §92, §90) — helpers, guard, restore, and the
# positive corpus check. Sourced by `plant.sh`; the plants are in `plants.d/`.
#
# §92: the gate "SHALL exit zero only when every positive fixture passes and
# every planted violation is REJECTED BY THE INTENDED CONTROL."
#
# That last clause is why this script checks WHICH rule fired, not merely that
# something failed. A malformed fixture rejected by the TOML parser instead of by
# the duplicate-ordinal rule proves nothing about the rule it was meant to
# exercise, while looking identical in a pass/fail summary.
#
# Every plant mutates the working tree and is restored with `git checkout`, so
# the script refuses to run unless the tree is clean.

set -uo pipefail

# No unquoted word-splitting anywhere in this file: a `for X in $LIST` loop does
# not split in zsh, and the silent no-op that produces has already cost this
# fleet a 13-repository operation that did nothing while reporting success.

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
WAR="./target/debug/war"

if [[ ! -x "$WAR" ]]; then
    echo "build first: cargo build --workspace" >&2
    exit 1
fi

# Only the tree the plants actually mutate has to be clean.
#
# Requiring a clean WORKING TREE would mean this step is skipped during ordinary
# development — and a gate step that is usually skipped is a gate step that is
# never run, which is the whole failure this battery exists to disprove. Editing
# Rust while the plants run is fine; editing docs/warrants/ is not, because the
# restore is `git checkout` and would discard that work. docs/gates/ joined this
# list when gate plants landed: the guard and the restore must name the same
# paths, or a plant silently deletes work the guard said it was protecting.
if ! git diff --quiet -- docs/warrants/ docs/adr/ docs/gates/ docs/sas/ docs/authority/ docs/roadmap/ docs/research/ openwarrant.toml \
    || ! git diff --cached --quiet -- docs/warrants/ docs/adr/ docs/gates/ docs/sas/ docs/authority/ docs/roadmap/ docs/research/ openwarrant.toml; then
    echo "docs/warrants/, docs/adr/, docs/gates/, docs/sas/, docs/authority/, docs/roadmap/, docs/research/ or openwarrant.toml has uncommitted changes." >&2
    echo "Plants mutate those files and restore with 'git checkout', which would" >&2
    echo "discard your work. Commit or stash those first." >&2
    exit 1
fi

PASSED=0
FAILED=0

restore() {
    git checkout -- docs/warrants/ docs/adr/ docs/gates/ docs/sas/ docs/authority/ docs/roadmap/ docs/research/ schemas/ openwarrant.toml 2>/dev/null || true
    # `git checkout` restores TRACKED files and leaves untracked ones behind, so
    # a plant that CREATES a file is not undone by it. AM-999 is exactly that —
    # the §91.4 test 24 positive fixture — and it leaked into a commit once
    # before this line existed. Named explicitly rather than `git clean`, which
    # would delete a developer's untracked work.
    rm -f docs/warrants/OW-WAR-0046/amendments/AM-999.yaml
    # `war plan --draft` without --apply leaves its scratch proposal behind.
    rm -f docs/warrants/generated/draft-proposal-*.json
    # The agent-acceptance plant proposes a throwaway SAS revision.
    rm -f docs/sas/revisions/0.1.0-draft.9.toml
    # The re-pin plants (88) fabricate a revision and an amendment each.
    rm -f docs/sas/revisions/9.9.9.toml docs/warrants/OW-WAR-0047/amendments/AM-901.yaml docs/warrants/OW-WAR-0062/amendments/AM-901.yaml
    rmdir docs/warrants/OW-WAR-0047/amendments docs/warrants/OW-WAR-0062/amendments 2>/dev/null || true
}

# The mirror of `assert_gone`, for a mutation that ADDS rather than removes.
# Same reason: a sed that matched nothing leaves the corpus valid and the plant
# then scores the untouched happy path.
assert_present() {
    if ! grep -Fq -- "$1" "$2"; then
        printf 'PLANT MUTATION WAS A NO-OP: %s never appeared in %s\n' "$1" "$2" >&2
        printf 'The plant would have scored the UNMUTATED corpus. Fix the pattern.\n' >&2
        restore
        exit 9
    fi
}

assert_gone() {
    if grep -Fq -- "$1" "$2"; then
        printf 'PLANT MUTATION WAS A NO-OP: %s still present in %s\n' "$1" "$2" >&2
        printf 'The plant would have scored the UNMUTATED corpus. Fix the pattern.\n' >&2
        restore
        exit 9
    fi
}

trap restore EXIT

# plant_cmd <name> <expected-rule> <expected-detail> <expected-exit> <mutation> <args...>
#
# The general form: run any `war` subcommand rather than `check` or `gate --run`.
# plant() and plant_gate() predate it and are kept because their call sites read
# better; new plants for new subcommands use this.
plant_cmd() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"
    shift 5

    restore
    eval "$mutate"

    local out status
    out="$("$WAR" "$@" 2>&1)"
    status=$?
    restore

    if [[ "$status" -ne "$want_exit" ]]; then
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$name" "$status" "$want_exit"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exited %s but %s never appeared\n' "$name" "$status" "$rule"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$detail" <<<"$out"; then
        printf 'FAIL  %-34s %s appeared but not for %s\n' "$name" "$rule" "$detail"
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s rejected by %s (%s)\n' "$name" "$rule" "$detail"
    PASSED=$((PASSED + 1))
}

# plant_gate <name> <expected-rule> <expected-detail> <expected-exit> <mutation>
#
# Same assertions as plant(), driving `war gate --run` instead of `war check`.
# §44's statuses are only observable when a gate is actually executed, and
# OW-WAR-0020's OBL-002 wants each status reported AS ITSELF — so the detail
# string carries the reason code, which is the thing that must not collapse.
plant_gate() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"

    restore
    eval "$mutate"

    local out status
    out="$("$WAR" gate --run 2>&1)"
    status=$?
    restore

    if [[ "$status" -ne "$want_exit" ]]; then
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$name" "$status" "$want_exit"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exited %s but rule %s never fired\n' "$name" "$status" "$rule"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$detail" <<<"$out"; then
        printf 'FAIL  %-34s rule %s fired but not for %s\n' "$name" "$rule" "$detail"
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s rejected by %s (%s)\n' "$name" "$rule" "$detail"
    PASSED=$((PASSED + 1))
}

# plant <name> <expected-rule> <expected-detail> <expected-exit> <mutation> [check-args...]
#
# `expected-detail` is what makes this a real check. Four of the plants below all
# surface under the rule `manifest.invalid`, so matching the rule alone would let
# a duplicate-ordinal plant "pass" while actually being caught by the
# unknown-role branch. The detail pattern pins the specific violation.
plant() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"
    shift 5

    restore
    eval "$mutate"

    local out status
    out="$("$WAR" check "$@" 2>&1)"
    status=$?
    restore

    if [[ "$status" -ne "$want_exit" ]]; then
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$name" "$status" "$want_exit"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exited %s but rule %s never fired\n' "$name" "$status" "$rule"
        printf '      (rejected for the wrong reason — the failure §92 warns about)\n'
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$detail" <<<"$out"; then
        printf 'FAIL  %-34s rule %s fired but not for %s\n' "$name" "$rule" "$detail"
        printf '      (right rule, wrong violation — still the wrong reason)\n'
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s rejected by %s (%s)\n' "$name" "$rule" "$detail"
    PASSED=$((PASSED + 1))
}

echo "== positive fixture =="
if "$WAR" check --generated >/dev/null 2>&1; then
    printf 'ok    %-34s clean corpus reports well-formed\n' "corpus"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s clean corpus does not pass\n' "corpus"
    FAILED=$((FAILED + 1))
fi

echo
echo "== planted violations =="

# The committed openwarrant.toml names a real drafter (OW-WAR-0042). A plant
# that needs NO drafter, or a fixture one, rewrites the `[plan]` table rather
# than appending a second (a duplicate table is a TOML error, not a plant) —
# and never lets the real agent run inside the battery.
plan_clear_drafter() {
    python3 - <<'PY'
import pathlib, re
p = pathlib.Path("openwarrant.toml"); s = p.read_text()
# Assumes no line inside [plan] starts with `[` (true of a table that is
# three scalar keys); a following table header is where the match stops.
s = re.sub(r'\n\[plan\]\n(?:(?!\[).*\n?)*', '\n', s)
p.write_text(s.rstrip("\n") + "\n")
PY
}
plan_set_drafter() {
    plan_clear_drafter
    printf '\n[plan]\n%s\n' "$1" >> openwarrant.toml
}
