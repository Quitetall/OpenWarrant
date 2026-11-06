#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Planted-violation battery (SAS §92, §90) — the runner.
#
# The helpers, the clean-tree guard, `restore`, and the positive corpus check
# live in `lib.sh`. The plants themselves live in `plants.d/NN-*.sh`, sourced
# in byte order so they share one PASSED/FAILED tally and one `restore`.
#
# Why the split: this file was one 1,200-line script pinned as a deliverable of
# OW-WAR-0063, so every Warrant that wanted to plant a refusal had to move a
# digest it did not own. Now a Warrant adds `plants.d/NN-<alias>.sh` and never
# touches this runner or another Warrant's plants. The `00-` file is the battery
# as it stood at the split, verbatim. NN is a grouping, not an id: two files
# may share it (lib.sh, "running the plant files").

# `plant.sh --shard i/n` runs one shard, the same as OW_PLANT_SHARD=i/n.
if [[ $# -eq 2 && "$1" == --shard ]]; then
    OW_PLANT_SHARD=$2
    shift 2
elif [[ $# -ne 0 ]]; then
    echo "usage: conformance/plant.sh [--shard i/n]" >&2
    exit 1
fi

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# lib.sh's positive corpus check is in the tally already, and runs once per
# process. A report names it apart, so shards added up count it once.
PLANT_FIXTURE_PASSED=$PASSED
PLANT_FIXTURE_FAILED=$FAILED

# An empty or missing plants.d/ is a broken battery, not a green one.
#
# The order is byte order of the names, and it does not decide the result: the
# NN- prefix is a grouping two files may share, and `run_plant_files` (lib.sh)
# starts each file from the restored tree and fails by name one that leaves a
# change behind (t-dc28).
mapfile -t PLANT_FILES < <(plant_files_in "$REPO_ROOT/conformance/plants.d")
if [[ ${#PLANT_FILES[@]} -eq 0 ]]; then
    echo "conformance/plants.d/ holds no plants; refusing to report a battery that ran nothing" >&2
    exit 1
fi
# OW_PLANT_SHARD=i/n runs one of n shards; OW_PLANT_FILES=<file> runs the plant
# files that file lists, one name per line (the PR tier, plant-select.py
# affected). Neither set is the whole battery, which is what `cargo xtask gate`
# runs. A selection is refused rather than read as "all", or as a smaller set
# than it says: a malformed selector, both at once, or a name no plant has.
#
# Sharding is a partition, not a sample: `plant-select.py shard` puts each file
# in exactly one shard, balanced by plant-weights.tsv. Each file starts from
# the restored tree, so a shard runs the same plants the whole does.
PLANT_SCOPE=""
PLANT_NAMES=""
if [[ -n "${OW_PLANT_SHARD:-}" && -n "${OW_PLANT_FILES:-}" ]]; then
    echo "OW_PLANT_SHARD and OW_PLANT_FILES each pick the plants; set one" >&2
    exit 1
fi
if [[ -n "${OW_PLANT_SHARD:-}" ]]; then
    if [[ ! "$OW_PLANT_SHARD" =~ ^([0-9]+)/([0-9]+)$ ]] \
        || (( BASH_REMATCH[1] < 1 || BASH_REMATCH[1] > BASH_REMATCH[2] || BASH_REMATCH[2] < 1 )); then
        echo "OW_PLANT_SHARD must be i/n with 1 <= i <= n, not '$OW_PLANT_SHARD'" >&2
        exit 1
    fi
    PLANT_NAMES=$(python3 "$REPO_ROOT/conformance/plant-select.py" shard "$OW_PLANT_SHARD") || {
        echo "plant-select.py could not name shard $OW_PLANT_SHARD; refusing to run a guess" >&2
        exit 1
    }
    PLANT_SCOPE="plant shard $OW_PLANT_SHARD"
elif [[ -n "${OW_PLANT_FILES:-}" ]]; then
    PLANT_NAMES=$(cat -- "$OW_PLANT_FILES") || {
        echo "OW_PLANT_FILES names '$OW_PLANT_FILES', which cannot be read" >&2
        exit 1
    }
    PLANT_SCOPE="selected plants"
fi
if [[ -n "$PLANT_SCOPE" ]]; then
    declare -A plant_known=()
    for f in "${PLANT_FILES[@]}"; do plant_known["$(basename "$f")"]="$f"; done
    picked=()
    while IFS= read -r name; do
        [[ -n "$name" ]] || continue
        if [[ -z "${plant_known[$name]+present}" ]]; then
            echo "$PLANT_SCOPE: '$name' is not a plant file in conformance/plants.d/" >&2
            exit 1
        fi
        picked+=("${plant_known[$name]}")
    done <<<"$PLANT_NAMES"
    echo "$PLANT_SCOPE: ${#picked[@]} of ${#PLANT_FILES[@]} plant file(s)"
    PLANT_FILES=(${picked[@]+"${picked[@]}"})
fi
run_plant_files ${PLANT_FILES[@]+"${PLANT_FILES[@]}"}
unset PLANT_ROOT


echo
echo "$PASSED passed, $FAILED failed"
[[ -z "$PLANT_SCOPE" ]] || echo "($PLANT_SCOPE: not the whole battery)"

# OW_PLANT_REPORT=<file>: what ran, for `plant-select.py reports`, which requires
# every plant exactly once across the shards. `total` is written last, so a
# shard that died part way has no complete report and fails that check.
if [[ -n "${OW_PLANT_REPORT:-}" ]]; then
    {
        printf 'shard %s\n' "${OW_PLANT_SHARD:-1/1}"
        printf 'fixture %s %s\n' "$PLANT_FIXTURE_PASSED" "$PLANT_FIXTURE_FAILED"
        for name in ${PLANT_RAN[@]+"${PLANT_RAN[@]}"}; do
            printf 'file %s %s\n' "$name" "${PLANT_SECONDS[$name]:-0}"
        done
        printf 'total %s %s\n' "$PASSED" "$FAILED"
    } > "$OW_PLANT_REPORT"
fi
[[ "$FAILED" -eq 0 ]]
