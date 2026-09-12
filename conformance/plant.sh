#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Planted-violation battery (SAS §92, §90) — the runner.
#
# The helpers, the clean-tree guard, `restore`, and the positive corpus check
# live in `lib.sh`. The plants themselves live in `plants.d/NN-*.sh`, sourced
# in sort order so they share one PASSED/FAILED tally and one `restore`.
#
# Why the split: this file was one 1,200-line script pinned as a deliverable of
# OW-WAR-0063, so every Warrant that wanted to plant a refusal had to move a
# digest it did not own. Now a Warrant adds `plants.d/NN-<alias>.sh` and never
# touches this runner or another Warrant's plants. The `00-` file is the battery
# as it stood at the split, verbatim.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# An empty or missing plants.d/ is a broken battery, not a green one.
shopt -s nullglob
PLANT_FILES=("$(dirname "${BASH_SOURCE[0]}")"/plants.d/*.sh)
shopt -u nullglob
if [[ ${#PLANT_FILES[@]} -eq 0 ]]; then
    echo "conformance/plants.d/ holds no plants; refusing to report a battery that ran nothing" >&2
    exit 1
fi
for plants in "${PLANT_FILES[@]}"; do
    # shellcheck source=/dev/null
    source "$plants"
done


echo
echo "$PASSED passed, $FAILED failed"
[[ "$FAILED" -eq 0 ]]
