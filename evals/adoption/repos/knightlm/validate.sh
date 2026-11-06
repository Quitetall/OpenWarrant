#!/usr/bin/env bash
# Prove the backlog invariants for the knightlm study repository.
#
#   1. On the seed alone, every item's check fails.
#   2. With reference patches order[0..=i] applied, item order[i]'s check passes,
#      so does every earlier item's, and `cargo test --workspace --locked` passes.
#   3. (v2) For an item with an [item.question]: earlier patches plus
#      reference/<ID>.default.patch (the item built with the natural default
#      instead of the answer) make that item's check FAIL while
#      `cargo test --workspace --locked` still passes. The default patch is then
#      reverted and <ID>.patch applied as in 2.
#
# Standalone, no arguments. Clones the source at the pinned commit into a fresh
# directory under /mnt/4tb/tmp, keeps all cargo output there, and removes it on exit.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRATCH_PARENT="${KNIGHTLM_VALIDATE_TMP:-/mnt/4tb/tmp}"
WORK="$(mktemp -d -p "$SCRATCH_PARENT" ow-adoption-knightlm-validate.XXXXXX)"
cleanup() { /usr/bin/rm -rf -- "$WORK"; }
trap cleanup EXIT

export CARGO_TARGET_DIR="$WORK/target"
export CARGO_BUILD_BUILD_DIR="$WORK/build"
export RUSTC_WRAPPER=
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export PYTHONDONTWRITEBYTECODE=1

read_field() { # read_field <toml> <top-level key>; lists print space-separated
  python3 -c 'import sys, tomllib; v = tomllib.load(open(sys.argv[1], "rb"))[sys.argv[2]]; print(" ".join(v) if isinstance(v, list) else v)' "$1" "$2"
}

SRC_PATH="$(read_field "$HERE/source.toml" path)"
SRC_COMMIT="$(read_field "$HERE/source.toml" commit)"
read -r -a ORDER <<< "$(read_field "$HERE/backlog.toml" order)"
REPO="$WORK/repo"

log() { printf '[validate %(%H:%M:%S)T] %s\n' -1 "$*"; }

git clone -q "$SRC_PATH" "$REPO"
git -C "$REPO" checkout -q --detach "$SRC_COMMIT"
log "seed $(git -C "$REPO" rev-parse HEAD)"

# Hidden tests live outside the repository, as the grader places them.
mkdir -p "$WORK/hidden"
for id in "${ORDER[@]}"; do
  /usr/bin/cp -r "$HERE/hidden/$id" "$WORK/hidden/$id"
  /usr/bin/rm -rf "$WORK/hidden/$id/__pycache__"
done

prepare() {
  local start=$SECONDS
  python3 - "$HERE/backlog.toml" "$REPO" <<'PY'
import subprocess, sys, tomllib
d = tomllib.load(open(sys.argv[1], "rb"))
argv = d["project"].get("prepare")
if argv:
    r = subprocess.run(argv, cwd=sys.argv[2], timeout=1800)
    sys.exit(r.returncode)
PY
  log "prepare ok ($((SECONDS - start))s)"
}

# run_check <id>: exit status of the item's check (124 on timeout).
run_check() {
  python3 - "$HERE/backlog.toml" "$1" "$REPO" "$WORK/hidden/$1" "$WORK/check-$1.log" <<'PY'
import subprocess, sys, tomllib
backlog, item_id, repo, hidden, logfile = sys.argv[1:]
item = next(i for i in tomllib.load(open(backlog, "rb"))["item"] if i["id"] == item_id)
argv = [a.replace("{hidden}", hidden).replace("{repo}", repo) for a in item["check"]]
with open(logfile, "w") as out:
    try:
        r = subprocess.run(argv, cwd=repo, stdout=out, stderr=subprocess.STDOUT,
                           timeout=item["timeout_secs"])
        code = r.returncode
    except subprocess.TimeoutExpired:
        code = 124
sys.exit(code)
PY
}

fail() { log "FAIL: $*"; exit 1; }

prepare
for id in "${ORDER[@]}"; do
  start=$SECONDS
  if run_check "$id"; then
    tail -5 "$WORK/check-$id.log"
    fail "$id check passes on the seed"
  fi
  log "seed: $id check fails as required ($((SECONDS - start))s; $(tail -1 "$WORK/check-$id.log"))"
done

has_question() { # has_question <id>: exit 0 when the item carries an [item.question]
  python3 - "$HERE/backlog.toml" "$1" <<'PY'
import sys, tomllib
item = next(i for i in tomllib.load(open(sys.argv[1], "rb"))["item"] if i["id"] == sys.argv[2])
sys.exit(0 if "question" in item else 1)
PY
}

snapshot() { # tree id of the working tree (the throwaway clone's index is scratch)
  git -C "$REPO" add -A && git -C "$REPO" write-tree
}

cargo_test() { # cargo_test <label>
  local start=$SECONDS
  if ! (cd "$REPO" && timeout 1800 cargo test --workspace --locked -q > "$WORK/test-$1.log" 2>&1); then
    tail -30 "$WORK/test-$1.log"
    fail "cargo test --workspace --locked fails after $1"
  fi
  log "  cargo test --workspace --locked passes ($((SECONDS - start))s)"
}

done_ids=()
for id in "${ORDER[@]}"; do
  if has_question "$id"; then
    default="$HERE/reference/$id.default.patch"
    [[ -f "$default" ]] || fail "$id has a question but no $id.default.patch"
    before="$(snapshot)"
    git -C "$REPO" apply --check "$default" || fail "$id.default.patch does not apply"
    git -C "$REPO" apply "$default"
    log "applied $id.default.patch ($(grep -c "^diff --git" "$default") files)"
    prepare
    start=$SECONDS
    if run_check "$id"; then
      tail -5 "$WORK/check-$id.log"
      fail "$id check passes with the natural default instead of the answer"
    fi
    log "  $id check fails on the default as required ($((SECONDS - start))s; $(tail -1 "$WORK/check-$id.log"))"
    cargo_test "$id.default"
    git -C "$REPO" apply -R "$default" || fail "$id.default.patch does not revert"
    [[ "$(snapshot)" == "$before" ]] || fail "reverting $id.default.patch did not restore the tree"
  fi
  patch="$HERE/reference/$id.patch"
  git -C "$REPO" apply --check "$patch" || fail "$id.patch does not apply"
  git -C "$REPO" apply "$patch"
  log "applied $id.patch ($(git apply --stat "$patch" | tail -1 | sed 's/^ //'); cumulative:$(git -C "$REPO" diff --shortstat HEAD))"
  prepare
  done_ids+=("$id")
  for earlier in "${done_ids[@]}"; do
    start=$SECONDS
    if ! run_check "$earlier"; then
      tail -30 "$WORK/check-$earlier.log"
      fail "$earlier check fails after applying patches through $id"
    fi
    log "  $earlier check passes ($((SECONDS - start))s; $(tail -1 "$WORK/check-$earlier.log"))"
  done
  cargo_test "$id"
done

log "disk: $(du -sh "$CARGO_TARGET_DIR" | cut -f1) target, $(du -sh "$CARGO_BUILD_BUILD_DIR" | cut -f1) build"
log "PASS: all ${#ORDER[@]} items satisfy the invariants"
