#!/usr/bin/env bash
# The fast tier, run here under CI's conditions, before a push.
#
#   bash conformance/ci-local.sh [BASE]     # BASE defaults to origin/main
#
# A PR's fast CI costs about ten minutes a round, and today's failures came from
# what differs between CI and a workstation, not from what was slow: no PyYAML,
# no harness session variable, a stale fixture. So this runs the same steps
# (`cargo xtask-fast`, then the plants the change selects) in a throwaway clone
# of HEAD merged onto BASE, as CI's pull-request checkout is, with a clean HOME,
# no session or agent variables, and a Python with only what CI installs.
#
# It needs cargo-deny and cargo-nextest, as CI installs. Committed work only:
# uncommitted changes are not in the clone, as they would not be on CI.
# Set CI_LOCAL_TMP to choose where the clone goes (default: $TMPDIR or /tmp).
set -uo pipefail

SRC=$(git rev-parse --show-toplevel) || exit 2
BASE=${1:-origin/main}
BASE_SHA=$(git -C "$SRC" rev-parse --verify "$BASE^{commit}") || { echo "ci-local: no commit $BASE"; exit 2; }
HEAD_SHA=$(git -C "$SRC" rev-parse HEAD)
ROOT=${CI_LOCAL_TMP:-${TMPDIR:-/tmp}}
for tool in cargo-deny cargo-nextest; do
    command -v "$tool" >/dev/null || { echo "ci-local: $tool is not installed (CI installs it)"; exit 2; }
done

WORK=$(mktemp -d "$ROOT/ow-ci-local.XXXXXX") || exit 2
[[ -n "$WORK" && "$WORK" == "$ROOT"/ow-ci-local.* ]] || exit 2
trap '[[ "$WORK" == "$ROOT"/ow-ci-local.* ]] && rm -rf -- "$WORK"' EXIT
CLONE="$WORK/repo"
git clone -q --no-hardlinks "$SRC" "$CLONE" || { echo "ci-local: clone failed"; exit 2; }
cd "$CLONE" || exit 2
[[ "$PWD" == "$CLONE" ]] || exit 2
git checkout -q --detach "$BASE_SHA" || exit 2
# The merge commit CI checks out: HEAD^1 is the base, HEAD^2 the change.
if ! git -c user.name=ci-local -c user.email=ci-local@invalid merge -q --no-ff --no-edit "$HEAD_SHA" >/dev/null 2>&1; then
    echo "ci-local: $HEAD_SHA does not merge cleanly onto $BASE; merge it first"
    exit 1
fi

# Python as CI has it: a venv without the system's packages, plus the one pin.
VENV="$WORK/venv"
python3 -m venv "$VENV" && PIP_DISABLE_PIP_VERSION_CHECK=1 "$VENV/bin/python" -m pip install -q 'jsonschema==4.26.0' \
    || { echo "ci-local: could not build the schema-validation venv"; exit 2; }

mkdir -p "$WORK/home" "$WORK/tmp"
CARGO_BIN=$(dirname "$(command -v cargo)")
# Only what CI's runner would have. Toolchain and build-dir settings pass
# through so a build reuses this machine's cache; nothing else does.
CLEAN=(env -i
    HOME="$WORK/home" TMPDIR="$WORK/tmp" CI=true LANG=C.UTF-8
    PATH="$VENV/bin:$CARGO_BIN:/usr/local/bin:/usr/bin:/bin"
    CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
    ${CARGO_BUILD_BUILD_DIR:+CARGO_BUILD_BUILD_DIR="$CARGO_BUILD_BUILD_DIR"}
    ${CARGO_TARGET_DIR:+CARGO_TARGET_DIR="$CARGO_TARGET_DIR"}
    OPENWARRANT_NO_UPDATE_CHECK=1)

status=0
step() {
    local name=$1; shift
    local t0=$SECONDS
    if "${CLEAN[@]}" "$@" >"$WORK/$name.log" 2>&1; then
        printf 'ok    %-10s %ss\n' "$name" $((SECONDS - t0))
    else
        printf 'FAIL  %-10s %ss  (last lines below)\n' "$name" $((SECONDS - t0))
        tail -n 25 "$WORK/$name.log" | sed 's/^/      /'
        status=1
    fi
}

echo "ci-local: $(git -C "$SRC" rev-parse --short HEAD) onto $BASE ($(git rev-parse --short "$BASE_SHA"))"
step lint cargo xtask-fast --only build,format,clippy,licenses,skills,doc-tests
step corpus cargo xtask-fast --only corpus,views,acceptance --base HEAD^1
step select python3 conformance/plant-select.py affected --base HEAD^1 --shard 1/1 --cap 1200 --out "$WORK/plants.txt"
# Views are written on main only (conformance/views.toml), so the clone's may
# lag the masters. As CI's tests and plants jobs do: compile them, and commit
# them locally (the battery restores with `git checkout`), after every step
# that reads HEAD^1.
step views python3 conformance/views.py fresh --war target/debug/war --commit
step tests cargo xtask-fast --only tests
step schemas cargo xtask-fast --only schemas
if [[ -s "$WORK/plants.txt" ]]; then
    echo "      $(wc -l <"$WORK/plants.txt") plant(s) selected"
    step plants env OW_PLANT_FILES="$WORK/plants.txt" bash conformance/plant.sh
    grep -aE '^FAIL|^[0-9]+ passed' "$WORK/plants.log" | head -20 | sed 's/^/      /'
fi
[[ $status -eq 0 ]] && echo "ci-local: PASS, as fast CI would see it" || echo "ci-local: FAIL"
exit $status
