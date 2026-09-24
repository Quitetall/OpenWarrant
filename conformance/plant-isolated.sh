#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The conformance battery, run in a disposable clone of the committed tree
# (OW-WAR-0145) — what `gate://ops.conformance.plants@1.1.0` executes.
#
# `conformance/plant.sh` mutates the working tree while it runs and restores
# it afterwards, which is why `@1.0.0` is `mutating` and, under §44.8 as
# `gate_cmd.rs` reads it, never askable. Here the battery runs in a clone of
# HEAD under the system temp directory; the repository's working tree, index
# and HEAD are never touched, so the gate is truthfully non-mutating.
#
# What it tests is HEAD — the committed tree. An uncommitted change is not
# tested, and the last line says which commit was.
#
# Environment:
#   OPENWARRANT_IN_BATTERY          set by this script for the battery it runs;
#                                   if already set, refuse rather than recurse
#   OPENWARRANT_ISOLATED_PLANT_SH   the battery script inside the clone
#                                   (default conformance/plant.sh); for this
#                                   script's own plants only
set -uo pipefail

if [[ -n "${OPENWARRANT_IN_BATTERY:-}" ]]; then
    echo "plant-isolated.refused-nested: already inside a battery run; a battery inside the battery would recurse" >&2
    exit 2
fi

repo=$(git -C "$(dirname "${BASH_SOURCE[0]}")/.." rev-parse --show-toplevel) || {
    echo "plant-isolated.not-a-repository: $(dirname "${BASH_SOURCE[0]}")/.. is not inside a git repository" >&2
    exit 2
}
commit=$(git -C "$repo" rev-parse HEAD) || exit 2
target=$(readlink -f "$repo/target" 2>/dev/null || true)

clone=$(mktemp -d "${TMPDIR:-/tmp}/war-isolated-battery.XXXXXX") || exit 2
cleanup() { rm -rf "$clone"; }
trap cleanup EXIT INT TERM

git clone -q --local --no-hardlinks "$repo" "$clone/r" 2>/dev/null \
    && git -C "$clone/r" checkout -q --detach "$commit" \
    || { echo "plant-isolated.clone-failed: could not clone $repo at $commit" >&2; exit 2; }

if [[ -n "$target" && -d "$target" ]]; then
    ln -s "$target" "$clone/r/target"
    export CARGO_TARGET_DIR="$target"
fi

battery="${OPENWARRANT_ISOLATED_PLANT_SH:-conformance/plant.sh}"
status=0
(cd "$clone/r" && OPENWARRANT_IN_BATTERY=1 bash "$battery") || status=$?
echo "tested commit $commit"
exit "$status"
