#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The battery once per ticket layout, each run in its own disposable clone of
# HEAD, reported per layout.
#
#   bash conformance/plant-layouts.sh [--layouts tickets,work] [PLANT.sh ...]
#   bash conformance/plant-layouts.sh --pins
#
# `war create` writes `docs/tickets/<t-id>.md` or `docs/work/<w-id>/`
# (`[tickets] layout`), and a plant must hold in both unless it is ABOUT one:
# such a file says `plant_layout <layout> "<reason>"` (lib.sh) and runs in that
# layout in every run. `--pins` lists those files with their reasons.
#
# With no PLANT.sh the whole battery runs per layout; with names, those plant
# files. The runs go side by side. Committed work only, as plant-isolated.sh:
# what is tested is HEAD, with this repository's built `target/`. The views are
# compiled and committed in each clone first (a branch carries none, and the
# battery restores with `git checkout`), as ci-local.sh does.
#
# Exit 0 only when every layout's run passed. A run that died is a failure of
# that layout, never a pass of the other.
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd) || exit 2
layouts=tickets,work
if [[ "${1:-}" == --pins ]]; then
    [[ $# -eq 1 ]] || { echo "usage: plant-layouts.sh --pins" >&2; exit 2; }
    # `plant_layout <layout> "<reason>"` at the start of a line is a pin.
    grep -HnE '^plant_layout ' "$here"/plants.d/*.sh \
        | sed -E 's|^.*/plants\.d/([^:]+):[0-9]+:plant_layout ([a-z]+) "?([^"]*)"?$|\2\t\1\t\3|' \
        | LC_ALL=C sort
    exit 0
fi
if [[ "${1:-}" == --layouts ]]; then
    [[ $# -ge 2 ]] || { echo "plant-layouts.sh: --layouts needs a list" >&2; exit 2; }
    layouts=$2
    shift 2
fi
IFS=, read -r -a LAYOUTS <<<"$layouts"
[[ ${#LAYOUTS[@]} -gt 0 ]] || { echo "plant-layouts.sh: --layouts names no layout" >&2; exit 2; }
for layout in "${LAYOUTS[@]}"; do
    [[ "$layout" == tickets || "$layout" == work ]] \
        || { echo "plant-layouts.sh: a layout is tickets or work, not '$layout'" >&2; exit 2; }
done
for name in "$@"; do
    [[ -f "$here/plants.d/$name" ]] \
        || { echo "plant-layouts.sh: '$name' is not a plant file in conformance/plants.d/" >&2; exit 2; }
done

if [[ -n "${OPENWARRANT_IN_BATTERY:-}" ]]; then
    echo "plant-layouts.refused-nested: already inside a battery run" >&2
    exit 2
fi
repo=$(git -C "$here" rev-parse --show-toplevel) || exit 2
commit=$(git -C "$repo" rev-parse HEAD) || exit 2
target=$(readlink -f "$repo/target" 2>/dev/null || true)
[[ -x "$target/debug/war" ]] || { echo "plant-layouts.sh: build first: cargo build --workspace" >&2; exit 2; }

work=$(mktemp -d "${TMPDIR:-/tmp}/war-layouts.XXXXXX") || exit 2
[[ -n "$work" && -d "$work" ]] || exit 2
cleanup() { rm -rf -- "$work"; }
trap cleanup EXIT INT TERM

# This run names its plants and its layouts itself.
unset OW_PLANT_SHARD OW_PLANT_FILES OW_PLANT_REPORT OW_PLANT_LAYOUT OPENWARRANT_TICKET_LAYOUT
# No harness session, as on CI: `war` records usage for a session it is run in,
# and a plant that reads the tree would see those files.
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID
if [[ $# -gt 0 ]]; then
    printf '%s\n' "$@" >"$work/plants.txt"
    export OW_PLANT_FILES="$work/plants.txt"
fi

# run_layout <layout>: the battery in a clone of its own; its exit status is
# written to <layout>.rc, and only by a run that reached its end.
run_layout() {
    local layout=$1 clone="$work/$1"
    {
        git clone -q --local --no-hardlinks "$repo" "$clone" \
            && git -C "$clone" checkout -q --detach "$commit" \
            && ln -s "$target" "$clone/target" \
            && cd "$clone" \
            && python3 conformance/views.py fresh --war target/debug/war --commit \
            || { echo "plant-layouts.clone-failed: no prepared clone of $commit for layout $layout"; return 2; }
        CARGO_TARGET_DIR="$target" bash conformance/plant.sh --layout "$layout"
        echo $? >"$work/$layout.rc"
    } >"$work/$layout.log" 2>&1
}
for layout in "${LAYOUTS[@]}"; do run_layout "$layout" & done
wait

status=0
for layout in "${LAYOUTS[@]}"; do
    rc=$(cat "$work/$layout.rc" 2>/dev/null || true)
    echo "== layout $layout =="
    grep -aE '^FAIL|^PLANT |^[0-9]+ passed' "$work/$layout.log" | cut -c1-400
    if [[ "$rc" == 0 ]]; then
        echo "layout $layout: PASS"
    else
        echo "layout $layout: FAIL (plant.sh exit ${rc:-none: the run did not finish})"
        [[ -n "$rc" ]] || tail -n 5 "$work/$layout.log" | cut -c1-400 | sed 's/^/    /'
        status=1
    fi
done
if [[ -n "${OW_PLANT_LAYOUT_LOGS:-}" ]]; then
    mkdir -p "$OW_PLANT_LAYOUT_LOGS" && cp "$work"/*.log "$OW_PLANT_LAYOUT_LOGS"/
fi
echo "tested commit $commit"
exit "$status"
