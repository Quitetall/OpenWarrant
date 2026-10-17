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

# `war` remembers every repository it opens (OW-WAR-0115). The battery opens
# dozens of scratch corpora that are gone a second later; none of them belongs
# in the list of whoever runs it. 71-hub.sh opts back in, against its own
# temporary XDG_CONFIG_HOME.
export OPENWARRANT_NO_PROJECTS=1

# No unquoted word-splitting anywhere in this file: a `for X in $LIST` loop does
# not split in zsh, and the silent no-op that produces has already cost this
# fleet a 13-repository operation that did nothing while reporting success.

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
REPO_ROOT="$PWD"
WAR="./target/debug/war"

# Every `war` a plant starts is inside this battery. `plant-isolated.sh` (the
# askable battery gate, OW-WAR-0145) refuses to start a battery inside one, so
# a `war gate --run` that reaches it from here refuses at once instead of
# cloning the repository and running this whole battery again under an hour's
# timeout. And no plant asks the network whether a newer war exists
# (OW-WAR-0143's notice): a battery's result does not depend on a release.
export OPENWARRANT_IN_BATTERY=1
export OPENWARRANT_NO_UPDATE_CHECK=1

# A battery gives the same answer whatever its caller exports (t-354e). Run
# with CLAUDE_PERFORMER_MODEL exported, 62-verifier's 'independence only where
# true' failed every time: its "unset" case read the caller's model. Every
# variable below is one `war`, the wrappers under tools/ or a fixture reads,
# and whose value changes what a plant observes; a plant that needs one sets
# it on the command it runs, never by inheriting it.
#
#   CLAUDE_*            the verifier, performer and drafter wrappers: the
#                       model (and so the independence claimed), the claude
#                       binary run, and where a run record is appended
#   OPENWARRANT_ACTOR   who a ticket act is recorded as
#   OPENWARRANT_FAULT*  atomic.rs's fault injection: a write that fails
#   OPENWARRANT_TEST_*  batch_cmd.rs's kill/fail-after test hooks
#   OPENWARRANT_RELEASES_URL  where `war update` looks
#   VISUAL, EDITOR      what `war sign --edit` runs
#   GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, ...  set when the battery is
#                       started from a git hook: every `git -C <scratch>`
#                       would then read the caller's repository instead
#   SSH_AUTH_SOCK, SSH_AGENT_PID  the caller's agent: a plant that signs brings
#                       its own agent and key, and none may reach the owner's
#
# Not unset, and why: HOME and XDG_CONFIG_HOME carry git's own configuration
# (the plants commit with `-c user.*`, and nothing they assert reads HOME);
# OPENWARRANT_NO_PROJECTS above already keeps XDG_CONFIG_HOME's project list
# out; PATH is how the plants find git and ssh-keygen. XDG_STATE_HOME (the web
# UI's device pairings), XDG_CACHE_HOME (the update notice) and XDG_DATA_HOME
# (`war install`) are pointed at a directory of this battery's own below the
# trap, so a plant never reads, or writes, the caller's pairings.
unset CLAUDE_PERFORMER_MODEL CLAUDE_VERIFIER_MODEL CLAUDE_VERIFIER_LOG CLAUDE_BIN \
    CLAUDE_PERFORMER_LOG CLAUDE_DRAFTER_MODEL CLAUDE_DRAFTER_LOG \
    OPENWARRANT_ACTOR OPENWARRANT_FAULT OPENWARRANT_FAULT_FILE \
    OPENWARRANT_TEST_BATCH_KILL_AFTER OPENWARRANT_TEST_BATCH_FAIL_AFTER \
    OPENWARRANT_RELEASES_URL VISUAL EDITOR \
    GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY \
    GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE GIT_PREFIX \
    SSH_AUTH_SOCK SSH_AGENT_PID

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

# Preserve all pre-existing Dispatch files, tracked or untracked. Only a
# Dispatch absent at battery entry is a disposable output of these plants.
declare -A INITIAL_DISPATCHES=()
while IFS= read -r -d '' path; do
    INITIAL_DISPATCHES["$path"]=1
done < <(git -C "$REPO_ROOT" ls-files --cached --others -z -- 'docs/warrants/*/dispatches/*.json')

PASSED=0
FAILED=0

restore() {
    local dispatch
    while IFS= read -r -d '' dispatch; do
        if [[ ! ${INITIAL_DISPATCHES["$dispatch"]+present} ]]; then
            /usr/bin/rm -f -- "$REPO_ROOT/$dispatch"
        fi
    done < <(git -C "$REPO_ROOT" ls-files --others -z -- 'docs/warrants/*/dispatches/*.json')
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

# scratch_queue <NAMESPACE>  ->  echoes the root of a program whose signing
# queue is not empty
#
# The plants that read "the signing queue" read this repository's until the
# owner signed every pending act on 2026-09-25 and seven of them failed at
# once: an empty queue is a true state of the corpus, and a plant that needs
# acts on the board has to bring its own. This one holds three acts of two
# kinds a plant can make without a key — two authorizations (the scaffold's
# adopt Warrant and one from `war new`) and a SAS acceptance (`war sas
# propose`) — and the example register as its register, so two humans are
# eligible and a plant that signs or dry-runs names one with `--as`.
#
# Committed, so `corpus_reset` returns it to exactly this. The caller removes
# it with `corpus_gone`: set from a command substitution, it never reaches
# SCRATCH_CORPORA.
scratch_queue() {
    local d n
    d=$(scratch_corpus "$1")
    [[ -d "${d:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
    cp "$d/docs/authority/roles.toml.example" "$d/docs/authority/roles.toml"
    cp "$d/docs/authority/allowed_signers.example" "$d/docs/authority/allowed_signers"
    "$WAR" --root "$d" new "A second pending authorization" >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: war new in %s\n' "$d" >&2; exit 9; }
    "$WAR" --root "$d" sas propose 0.1.0 >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: war sas propose in %s\n' "$d" >&2; exit 9; }
    "$WAR" --root "$d" compile >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: war compile in %s\n' "$d" >&2; exit 9; }
    git -C "$d" add -A >/dev/null 2>&1
    git -C "$d" -c user.email=plant@invalid -c user.name=plant commit -qm "register, a queue of three" >/dev/null 2>&1 \
        || { printf 'PLANT SETUP FAILED: queue commit in %s\n' "$d" >&2; exit 9; }
    n=$("$WAR" --root "$d" sign --list 2>/dev/null | grep -cE '^  ([A-Z]+-WAR-[0-9]{4}|SAS [0-9]+\.[0-9]+\.[0-9]+) ')
    [[ "$n" -eq 3 ]] || { printf 'PLANT SETUP FAILED: wanted three pending acts in %s, have %s\n' "$d" "$n" >&2; exit 9; }
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

# The battery's own XDG state, cache and data (t-354e, above), removed with
# the scratch corpora.
BATTERY_XDG=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$BATTERY_XDG")
export XDG_STATE_HOME="$BATTERY_XDG/state" XDG_CACHE_HOME="$BATTERY_XDG/cache" \
    XDG_DATA_HOME="$BATTERY_XDG/data"

# battery_tool <dest-dir> <binary> <cargo build args...>
#
# Build a tool the battery needs and `./target/debug/war` is not (the schema
# generator behind the `schema` feature; xtask), and copy its binary to
# <dest-dir>/<binary>, a path only this battery runs. Returns non-zero, with
# cargo's output on stderr, when the build fails.
#
# Why not `cargo run` (t-d052): the battery gate runs each battery in its own
# clone with CARGO_TARGET_DIR pointing at ONE shared build directory. A
# `cargo run` from a clone sees sources at a new path, rebuilds, and replaces
# `<target>/debug/war` by unlink-then-link while every other battery is
# executing that path: a battery that starts `war` in the gap gets exit 127, or
# empty output where it wanted JSON. And a `--features schema` build left the
# schema-featured war, with whatever pack a plant had planted, as the binary
# every later plant ran. So the tools build into a directory of their own,
# `<target>/battery-tools`, never the one `$WAR` lives in, under a lock so
# two batteries do not copy a binary out while the other relinks it; the copy
# is what runs. What a tool reads comes from its working directory (this
# tree), not from where it was built.
battery_tool() {
    local dest="$1" bin="$2" tools
    shift 2
    tools="$(cd "$REPO_ROOT" && readlink -f "${CARGO_TARGET_DIR:-target}")/battery-tools" || return 1
    mkdir -p "$tools" || return 1
    (
        # flock where it exists (util-linux); without it, cargo's own lock
        # still serialises the builds and only the copy can race.
        if command -v flock >/dev/null 2>&1; then flock 9 || exit 1; fi
        CARGO_TARGET_DIR="$tools" cargo build -q "$@" >&2 || exit 1
        command cp "$tools/debug/$bin" "$dest/$bin"
    ) 9>"$tools/.lock"
}

# ---- running the plant files (t-dc28) --------------------------------------
#
# `plants.d/NN-<name>.sh`: the NN is a GROUPING, not an id. Branches add plant
# files without coordinating, so two files sharing a prefix (57-grammar.sh and
# 57-tickets-ui.sh) is normal and harmless. What makes that true:
#
# - The order is byte order of the file names (LC_ALL=C), the same on every
#   machine and locale: files sharing a prefix run in name order, and the
#   battery is reproducible.
# - No file's correctness depends on that order. Each file starts from the
#   restored tree, and the tree is restored after it; a file that leaves a
#   change `restore` cannot undo — a new file under a plant path — is FAILED
#   by name and what it created is removed, so it can neither pass nor break
#   the files after it. Shell variables a file sets are its own business: a
#   file reads none it did not set itself, except what lib.sh defines.
#
# A new plant file needs no free number: pick the group it belongs to.

# line_has <filter-flag> <filter-pattern> <test-flag> <test-pattern> <<<"$text"
#
# True when some line of the text matches the filter and, among the lines that
# do, one matches the test: what `grep F P <<<"$text" | grep -qT Q` meant,
# without the pipe. Each flag is one grep option word (-G for a basic regex,
# -E, -F, -xF, -v, -i, -A1, ...); the patterns are never read as options.
#
# Not `grep … | grep -q …` (t-515e): under `set -o pipefail` the second grep
# exits at its first match, and when the first has more than one pipe
# buffer's worth of lines to write, its next write takes SIGPIPE (141) and the
# pipeline reads false with the line plainly there. 69-standing's
# 'propose refuses **' failed that way (t-0f24). Here the filter's lines are
# captured whole, then tested. A filter that matches nothing is false, as the
# pipeline was; the test reads exactly the filter's lines, none added.
line_has() {
    local _lh_lines
    _lh_lines=$(grep "$1" -- "$2" && printf x) || return 1
    _lh_lines=${_lh_lines%x}
    grep -q "$3" -- "$4" <<<"${_lh_lines%$'\n'}"
}

# plant_quiet_grep_pipes <file>: each line of <file> that pipes into a quiet
# grep (`| grep -q`, `-qF`, `-Fq`, `--quiet`, `|& grep -E -q` ...), as
# "N: <line>"; nothing when there is none. A pipe whose `|` ends one line and
# whose grep starts the next is named at the grep's line.
#
# Under `set -o pipefail` (set above), `producer | grep -q X` is true only
# when the producer also exits 0, and `grep -q` exits at its first match: a
# producer with more than one pipe buffer still to write takes SIGPIPE and
# the check reads false, or, negated, true (t-515e). Capture the producer's
# output first and grep that; `line_has` for grep into grep. A comment line
# may name the pattern; `||` is not a pipe.
plant_quiet_grep_pipes() {
    awk '
        function quiet(s) {
            sub(/^[[:space:]]+/, "", s)
            return s ~ /^grep([[:space:]]+-[^[:space:]]*)*[[:space:]]+(-[A-Za-z0-9]*q[A-Za-z0-9]*|--quiet|--silent)([[:space:]]|$)/
        }
        /^[[:space:]]*#/ { next }
        {
            s = $0
            gsub(/\|\|/, "\001\001", s)
            gsub(/\|&/, "|", s)
            n = split(s, seg, "|")
            hit = (open && quiet(seg[1]))
            for (i = 2; i <= n && !hit; i++) if (quiet(seg[i])) hit = 1
            if (hit) printf "%d: %s\n", NR, $0
            open = (s ~ /\|[[:space:]]*\\?[[:space:]]*$/)
        }' "$1"
}

# plant_quiet_grep_held <file>: true when <file> is one whose exact bytes an
# obligation holds, so it may not be converted: its name and sha256 are below.
# Any other bytes under that name are scanned like every file.
#
#   63-webui.sh  OW-WAR-0139 OBL-005: "63-webui.sh passes unmodified"; 59-webui-lan
#                checks this digest. Its three quiet-grep pipes (ss | awk, and the
#                source grep) stay until an amendment of OW-WAR-0139 releases it.
PLANT_QUIET_GREP_HELD=(
    "63-webui.sh 0397209b30e60b7aab726f1742ba7556c844e671f2df1a81692cd0d9ac376a79"
)
plant_quiet_grep_held() {
    local _qh_entry _qh_sum
    _qh_sum=$(sha256sum "$1" | cut -d' ' -f1)
    for _qh_entry in "${PLANT_QUIET_GREP_HELD[@]}"; do
        [[ "$_qh_entry" == "$(basename "$1") $_qh_sum" ]] && return 0
    done
    return 1
}

# plant_files_in <dir>: the plant files, one per line, in byte order.
plant_files_in() {
    local LC_ALL=C
    local -a files
    shopt -s nullglob
    files=("$1"/*.sh)
    shopt -u nullglob
    [[ ${#files[@]} -eq 0 ]] || printf '%s\n' "${files[@]}"
}

# What the plant paths hold, tracked and untracked, for the leak check.
plant_tree_state() {
    git -C "$REPO_ROOT" status --porcelain --untracked-files=all -- "${PLANT_PATHS[@]}"
}

# plant_fixed_tmp_paths <file>: each line of <file> that names a fixed path
# under /tmp, as "N: <line>"; nothing when there is none.
#
# Batteries run side by side (one `ops.conformance.plants@1.1.0` gate per
# Warrant, each in its own clone), and a fixed path under /tmp is the one
# thing their clones still share: two batteries overwrite each other's file
# between a write and the read that follows it (t-0e4f: plant 64's correction
# response). A plant's scratch file belongs in a directory from `mktemp -d`.
# What counts: a path that STARTS at /tmp/ (not .../tmp/ inside another
# path, not `${TMPDIR:-/tmp}/`, which is the temp root and never fixed), on a
# line that is not a comment, unless the path is a mktemp template (XXX).
plant_fixed_tmp_paths() {
    awk '
        /^[[:space:]]*#/ { next }
        {
            s = $0
            while (match(s, /(^|[^A-Za-z0-9_.}\/~-])\/tmp\/[^[:space:]"'"'"'`;)|&<>]*/)) {
                if (substr(s, RSTART, RLENGTH) !~ /XXX/) { printf "%d: %s\n", NR, $0; break }
                s = substr(s, RSTART + RLENGTH)
            }
        }' "$1"
}

# run_plant_files <file...>: source each in turn under the rules above. A file
# that names a fixed /tmp path, or that pipes into a quiet grep, is not run: it
# is FAILED by name, with the line (plant_fixed_tmp_paths,
# plant_quiet_grep_pipes; plant_quiet_grep_held names the one exception).
run_plant_files() {
    local _rpf_file _rpf_base _rpf_now _rpf_leak _rpf_line _rpf_tmp _rpf_pipe
    local -a _rpf_files=("$@")
    # A sourced file would see these positional parameters; it gets none.
    set --
    _rpf_base=$(plant_tree_state)
    for _rpf_file in ${_rpf_files[@]+"${_rpf_files[@]}"}; do
        # Per FILE, never inherited: a plant file that forgets to unset
        # PLANT_ROOT would otherwise silently redirect the next file's plants at
        # its own scratch, and they would pass against a tree they never meant
        # to test.
        unset PLANT_ROOT
        cd "$REPO_ROOT" || exit 1
        _rpf_tmp=$(plant_fixed_tmp_paths "$_rpf_file")
        if [[ -n "$_rpf_tmp" ]]; then
            printf 'FAIL  %-34s names a fixed /tmp path, so it was not run (use mktemp -d): %s\n' \
                "$(basename "$_rpf_file")" "$(head -1 <<<"$_rpf_tmp")"
            FAILED=$((FAILED + 1))
            continue
        fi
        _rpf_pipe=$(plant_quiet_grep_pipes "$_rpf_file")
        if [[ -n "$_rpf_pipe" ]] && ! plant_quiet_grep_held "$_rpf_file"; then
            printf 'FAIL  %-34s pipes into a quiet grep, so it was not run (capture first; line_has): %s\n' \
                "$(basename "$_rpf_file")" "$(head -1 <<<"$_rpf_pipe")"
            FAILED=$((FAILED + 1))
            continue
        fi
        restore
        # shellcheck source=/dev/null
        source "$_rpf_file"
        unset PLANT_ROOT
        cd "$REPO_ROOT" || exit 1
        restore
        _rpf_now=$(plant_tree_state)
        [[ "$_rpf_now" == "$_rpf_base" ]] && continue
        _rpf_leak=$(comm -13 <(sort <<<"$_rpf_base") <(sort <<<"$_rpf_now"))
        printf 'FAIL  %-34s left the tree changed after restore: %s\n' \
            "$(basename "$_rpf_file")" "$(tr '\n' ' ' <<<"$_rpf_leak")"
        FAILED=$((FAILED + 1))
        while IFS= read -r _rpf_line; do
            [[ "$_rpf_line" == '?? '* ]] && rm -f -- "$REPO_ROOT/${_rpf_line#?? }"
        done <<<"$_rpf_leak"
    done
}

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
    # The gate every plant_gate mutates, by name: `gate --run` with none runs
    # every askable gate, the battery gate among them.
    out="$(plant_war gate --run --gate software.repo.war-check@1.0.0 2>&1)"
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
