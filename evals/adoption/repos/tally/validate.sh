#!/usr/bin/env bash
# Prove the backlog invariants for the tally study repository.
#
#   1. On the seed alone, every item's check fails.
#   2. Before patch i is applied (patches 0..i-1 applied), item i's check fails.
#   3. After patches 0..i are applied, items 0..i pass and the visible
#      test suite passes.
#   4. For an item with an open question ([item.question]): with patches
#      0..i-1 plus reference/<ID>.default.patch (the item built with the
#      natural default instead of the answer), the item's check fails and the
#      visible tests pass. That tree is then discarded and <ID>.patch applied.
#
# Takes no arguments. Works in a temporary directory under /mnt/4tb/tmp and
# removes it on exit. Exit status 0 only if every assertion holds.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRATCH_ROOT=/mnt/4tb/tmp
WORK="$(mktemp -d "$SCRATCH_ROOT/tally-validate.XXXXXX")"
trap '/usr/bin/rm -rf "$WORK"' EXIT

REPO="$WORK/repo"
HIDDEN="$WORK/hidden"
export PYTHONDONTWRITEBYTECODE=1
export GIT_AUTHOR_NAME=validate GIT_AUTHOR_EMAIL=validate@example.invalid
export GIT_COMMITTER_NAME=validate GIT_COMMITTER_EMAIL=validate@example.invalid

failures=0
pass() { printf '  ok    %s\n' "$*"; }
fail() { printf '  FAIL  %s\n' "$*"; failures=$((failures + 1)); }

/usr/bin/cp -r "$HERE/seed" "$REPO"
/usr/bin/cp -r "$HERE/hidden" "$HIDDEN"
git -C "$REPO" init -q
git -C "$REPO" add -A
git -C "$REPO" commit -qm seed

# One line per item, in `order`: ID <TAB> timeout <TAB> question? (0/1) <TAB> shell-quoted check.
mapfile -t ITEMS < <(python3 - "$HERE/backlog.toml" "$HIDDEN" "$REPO" <<'PY'
import shlex, sys, tomllib
backlog_path, hidden, repo = sys.argv[1:]
with open(backlog_path, "rb") as f:
    backlog = tomllib.load(f)
items = {item["id"]: item for item in backlog["item"]}
order = backlog["order"]
assert sorted(order) == sorted(items), "order must list every item exactly once"
seen = set()
for item_id in order:
    deps = items[item_id]["depends_on"]
    assert set(deps) <= seen, f"{item_id} comes before its dependency {deps}"
    seen.add(item_id)
for item_id in order:
    item = items[item_id]
    question = item.get("question")
    if question:
        text = f"Open question: {question['asked'].strip()} The answer will be added to this item; follow it."
        assert item["description"].rstrip().endswith(text), f"{item_id}: description must end with the open question"
        assert question["answer"].strip() not in item["description"], f"{item_id}: answer leaks into the description"
    argv = [a.replace("{hidden}", f"{hidden}/{item_id}").replace("{repo}", repo) for a in item["check"]]
    print(f"{item_id}\t{item['timeout_secs']}\t{1 if question else 0}\t{shlex.join(argv)}")
PY
)

ids=()
questions=()
for line in "${ITEMS[@]}"; do
    ids+=("${line%%$'\t'*}")
    rest="${line#*$'\t'}"; rest="${rest#*$'\t'}"
    questions+=("${rest%%$'\t'*}")
done
printf 'items in order: %s\n' "${ids[*]}"

run_check() { # index -> exit status of that item's check, run in the repo
    local line="${ITEMS[$1]}" rest timeout cmd
    rest="${line#*$'\t'}"
    timeout="${rest%%$'\t'*}"
    cmd="${rest#*$'\t'}"
    cmd="${cmd#*$'\t'}"
    (cd "$REPO" && eval "timeout $timeout $cmd" >/dev/null 2>&1)
}

run_visible() {
    (cd "$REPO" && timeout 300 python3 -m pytest -q -p no:cacheprovider tests >/dev/null 2>&1)
}

echo "seed:"
if run_visible; then pass "visible tests pass on the seed"; else fail "visible tests fail on the seed"; fi
for i in "${!ids[@]}"; do
    if run_check "$i"; then fail "${ids[$i]} check passes on the seed"; else pass "${ids[$i]} check fails on the seed"; fi
done

for i in "${!ids[@]}"; do
    id="${ids[$i]}"
    echo "$id:"
    if run_check "$i"; then fail "$id check passes before its patch"; else pass "$id check fails before its patch"; fi
    if [ "${questions[$i]}" = 1 ]; then
        default="$HERE/reference/$id.default.patch"
        if [ ! -f "$default" ]; then
            fail "reference/$id.default.patch is missing"
        elif git -C "$REPO" apply --check "$default" 2>/dev/null; then
            git -C "$REPO" apply "$default"
            pass "reference/$id.default.patch applies"
            if run_check "$i"; then fail "$id check passes with the default instead of the answer"; else pass "$id check fails with the default instead of the answer"; fi
            if run_visible; then pass "visible tests pass with the default"; else fail "visible tests fail with the default"; fi
        else
            fail "reference/$id.default.patch does not apply"
        fi
        git -C "$REPO" reset -q --hard HEAD
        git -C "$REPO" clean -qfdx
    fi
    if git -C "$REPO" apply --check "$HERE/reference/$id.patch" 2>/dev/null; then
        git -C "$REPO" apply "$HERE/reference/$id.patch"
        git -C "$REPO" add -A
        git -C "$REPO" commit -qm "$id"
        pass "reference/$id.patch applies"
    else
        fail "reference/$id.patch does not apply"
        continue
    fi
    for j in $(seq 0 "$i"); do
        if run_check "$j"; then pass "${ids[$j]} check passes"; else fail "${ids[$j]} check fails"; fi
    done
    if run_visible; then pass "visible tests pass"; else fail "visible tests fail"; fi
done

if [ "$failures" -eq 0 ]; then
    echo "validate: all invariants hold"
else
    echo "validate: $failures assertion(s) failed"
    exit 1
fi
