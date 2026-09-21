#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
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
REPO_ROOT="$PWD"
WAR="./target/debug/war"

if [[ ! -x "$WAR" ]]; then
    echo "build first: cargo build --workspace" >&2
    exit 1
fi

# Every path the plants mutate, named ONCE.
#
# The guard below and `restore` must name the same set. They were two lists kept
# by hand and they had already drifted apart in both directions: `schemas/` was
# in the restore and not the guard, so uncommitted work there was discarded by a
# battery run while the guard said nothing; `docs/SKILLS.md` was in NEITHER, so
# the pins plant appended to it and nothing put it back — twice into a commit,
# which is where this corpus's `deliverable.digest-drift` on OW-WAR-0068 D-009
# comes from. One array, read by both, so they cannot disagree again.
PLANT_PATHS=(
    docs/warrants/
    docs/adr/
    docs/gates/
    docs/sas/
    docs/authority/
    docs/roadmap/
    docs/research/
    docs/SKILLS.md
    schemas/
    openwarrant.toml
)

# Only the tree the plants actually mutate has to be clean.
#
# Requiring a clean WORKING TREE would mean this step is skipped during ordinary
# development — and a gate step that is usually skipped is a gate step that is
# never run, which is the whole failure this battery exists to disprove. Editing
# Rust while the plants run is fine; editing docs/warrants/ is not, because the
# restore is `git checkout` and would discard that work.
if ! git diff --quiet -- "${PLANT_PATHS[@]}" \
    || ! git diff --cached --quiet -- "${PLANT_PATHS[@]}"; then
    echo "A path the plants mutate has uncommitted changes:" >&2
    printf '  %s\n' "${PLANT_PATHS[@]}" >&2
    echo "Plants mutate those files and restore with 'git checkout', which would" >&2
    echo "discard your work. Commit or stash those first." >&2
    exit 1
fi

PASSED=0
FAILED=0

restore() {
    git -C "$REPO_ROOT" checkout -- "${PLANT_PATHS[@]}" 2>/dev/null || true
    # `git checkout` restores TRACKED files and leaves untracked ones behind, so
    # a plant that CREATES a file is not undone by it. AM-999 is exactly that —
    # the §91.4 test 24 positive fixture — and it leaked into a commit once
    # before this line existed. Named explicitly rather than `git clean`, which
    # would delete a developer's untracked work.
    rm -f "$REPO_ROOT"/docs/warrants/OW-WAR-0046/amendments/AM-999.yaml
    # `war plan --draft` without --apply leaves its scratch proposal behind.
    rm -f "$REPO_ROOT"/docs/warrants/generated/draft-proposal-*.json
    # The agent-acceptance plant proposes a throwaway SAS revision.
    rm -f "$REPO_ROOT"/docs/sas/revisions/0.1.0-draft.9.toml
    # The re-pin plants (88) fabricate a revision and an amendment each.
    rm -f "$REPO_ROOT"/docs/sas/revisions/9.9.9.toml "$REPO_ROOT"/docs/warrants/OW-WAR-0047/amendments/AM-901.yaml "$REPO_ROOT"/docs/warrants/OW-WAR-0062/amendments/AM-901.yaml
    rmdir "$REPO_ROOT"/docs/warrants/OW-WAR-0047/amendments "$REPO_ROOT"/docs/warrants/OW-WAR-0062/amendments 2>/dev/null || true
}

# The mirror of `assert_gone`, for a mutation that ADDS rather than removes.
# Same reason: a sed that matched nothing leaves the corpus valid and the plant
# then scores the untouched happy path.
assert_present() {
    if ! grep -Fq -- "$1" "$2"; then
        printf 'PLANT MUTATION WAS A NO-OP: %s never appeared in %s\n' "$1" "$2" >&2
        printf 'The plant would have scored the UNMUTATED corpus. Fix the pattern.\n' >&2
        plant_restore
        exit 9
    fi
}

# The same guard for a mutation that REMOVES a file rather than a line. A plant
# that deletes a signature proves nothing if the signature was not there.
assert_gone_file() {
    if [[ -e "$1" ]]; then
        printf 'PLANT MUTATION WAS A NO-OP: %s still exists\n' "$1" >&2
        printf 'The plant would have scored the UNMUTATED corpus. Fix the pattern.\n' >&2
        plant_restore
        exit 9
    fi
}

assert_gone() {
    if grep -Fq -- "$1" "$2"; then
        printf 'PLANT MUTATION WAS A NO-OP: %s still present in %s\n' "$1" "$2" >&2
        printf 'The plant would have scored the UNMUTATED corpus. Fix the pattern.\n' >&2
        plant_restore
        exit 9
    fi
}

# A throwaway program for a plant to break, instead of this repository.
#
# `scratch_warrant` above exists because plants named real Warrants and broke
# the night the owner signed one — four at once on 2026-09-13. This is the same
# lesson one level up: the tree a plant mutates should be one nobody owns. It
# also buys what the live corpus cannot give a plant — a clean `war check`, and
# a target under `crates/` — because the reset is `git reset --hard` on a
# scaffold rather than a list of paths somebody has to keep correct.
#
# A plant file opts in by setting PLANT_ROOT; every helper below then redirects
# to it. Unset, which is what every plant file does today, nothing changes.
SCRATCH_CORPORA=()

# scratch_corpus <NAMESPACE>  ->  echoes the root of a fresh program
scratch_corpus() {
    local ns="$1" d
    d=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
    ( cd "$d" && git init -q . ) \
        || { printf 'PLANT SETUP FAILED: git init in %s\n' "$d" >&2; exit 9; }
    "$WAR" init --program "Plant Corpus $ns" --namespace "$ns" --root "$d" >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: war init --program in %s\n' "$d" >&2; exit 9; }
    "$WAR" --root "$d" compile >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: war compile in %s\n' "$d" >&2; exit 9; }
    git -C "$d" add -A >/dev/null 2>&1
    git -C "$d" -c user.email=plant@invalid -c user.name=plant commit -qm baseline >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: baseline commit in %s\n' "$d" >&2; exit 9; }
    SCRATCH_CORPORA+=("$d")
    printf '%s' "$d"
}

corpus_reset() { git -C "$1" reset --hard -q && git -C "$1" clean -fdq; }

corpus_gone() {
    [[ -n "${1:-}" ]] || return 0
    rm -rf "$1"
}

scratch_corpora_gone() {
    local d
    for d in ${SCRATCH_CORPORA[@]+"${SCRATCH_CORPORA[@]}"}; do
        rm -rf "$d"
    done
}

# Undo what the current plant mutated: its scratch program if it declared one,
# this repository otherwise.
plant_restore() {
    if [[ -n "${PLANT_ROOT:-}" ]]; then
        corpus_reset "$PLANT_ROOT"
    else
        restore
    fi
}

# Run the mutation where its target lives. Deliberately NOT a subshell: the
# assert_* guards call `exit 9` to abort the whole battery when a mutation was
# a no-op, and a subshell would swallow that into an ordinary failure. The cwd
# is therefore left in the scratch on that path, which is why `restore` is
# anchored to REPO_ROOT above.
plant_mutate() {
    if [[ -n "${PLANT_ROOT:-}" ]]; then
        cd "$PLANT_ROOT" || return 1
        eval "$1"
        local rc=$?
        cd "$REPO_ROOT" || return 1
        return "$rc"
    fi
    eval "$1"
}

plant_war() {
    if [[ -n "${PLANT_ROOT:-}" ]]; then
        "$WAR" --root "$PLANT_ROOT" "$@"
    else
        "$WAR" "$@"
    fi
}

trap 'restore; scratch_corpora_gone' EXIT

# scratch_warrant <what-for>  ->  echoes a fresh alias awaiting authorization
#
# A plant that needs "a Warrant nobody has signed yet" used to name one from the
# corpus. Every such plant broke the night the owner signed that Warrant — four
# of them at once on 2026-09-13 — because the corpus is supposed to move and the
# plants were pinned to a moment in it. A scaffold from `war new` is well-formed,
# awaits its first authorization, and belongs to the plant that made it.
#
# The caller removes it with `scratch_warrant_gone` on every exit path, including
# the failing ones: an untracked directory is not undone by `git checkout`.
scratch_warrant() {
    local out alias
    out=$("$WAR" new "Plant scratch: ${1:-a plant}" 2>&1) || {
        printf 'PLANT SETUP FAILED: could not create a scratch Warrant\n%s\n' "$out" >&2
        exit 9
    }
    alias=$(grep -oE '[A-Z][A-Z0-9-]*-WAR-[0-9]{4}' <<< "$out" | head -1)
    if [[ -z "$alias" ]]; then
        printf 'PLANT SETUP FAILED: `war new` named no alias\n%s\n' "$out" >&2
        exit 9
    fi
    printf '%s' "$alias"
}

scratch_warrant_gone() {
    [[ -n "${1:-}" ]] || return 0
    rm -rf "docs/warrants/$1"
    rm -f docs/authority/responses/"$1".* 2>/dev/null || true
}

# plant_cmd <name> <expected-rule> <expected-detail> <expected-exit> <mutation> <args...>
#
# The general form: run any `war` subcommand rather than `check` or `gate --run`.
# plant() and plant_gate() predate it and are kept because their call sites read
# better; new plants for new subcommands use this.
plant_cmd() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"
    shift 5

    plant_restore
    plant_mutate "$mutate" || { FAILED=$((FAILED + 1)); return; }

    local out status
    out="$(plant_war "$@" 2>&1)"
    status=$?
    plant_restore

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

    plant_restore
    plant_mutate "$mutate" || { FAILED=$((FAILED + 1)); return; }

    local out status
    out="$(plant_war gate --run 2>&1)"
    status=$?
    plant_restore

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

    plant_restore
    plant_mutate "$mutate" || { FAILED=$((FAILED + 1)); return; }

    local out status
    out="$(plant_war check "$@" 2>&1)"
    status=$?
    plant_restore

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
