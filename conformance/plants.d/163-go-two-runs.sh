# shellcheck shell=bash
# OW-WAR-0148 M15 — two `war evidence go` runs at once, in two worktrees of
# one clone, never run one node twice (M11's shared claims, and the landed
# markers beside them under git's common directory).
#
# One scratch program (GW) with eight independent items, and a second
# worktree of it. Item X is claimed first by someone else (`war claim --as
# someone`). Both runs start together, two nodes at a time each, with the
# fixture performer (no model) logging every start to one shared log. Then a
# third run in the second worktree.
#
# Accepted: across all three runs every item but X started exactly once and
# landed; the third run, whose worktree's checklist shows none of the first
# run's ticks, starts nothing that already landed.
# Refused: X, held by someone else, is started by no run, and each run's
# stop says one node is held by someone else.

echo "== war go: two runs never run one node twice (M15) =="
GW_ROOT=$(scratch_corpus GW)
[[ -d "${GW_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
GW_TMP=$(mktemp -d)
GW_WAR="$REPO_ROOT/${WAR#./}"
GW_FIX="$REPO_ROOT/conformance/fixtures/go/performer.py"
gw_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
gw_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
gww() { local root=$1; shift; env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID GO_LOG="$GW_TMP/go.log" \
    GO_SLEEP=0.4 "$GW_WAR" --root "$root" "$@" </dev/null; }
gw_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }

GW_NODES=()
for n in 1 2 3 4 5 6 7 8; do
    GW_NODES+=("$(gw_field "$(gww "$GW_ROOT" --json create "Independent $n" -i "w$n" 2>/dev/null)" \
        'v["result"]["id"] + "/" + v["result"]["items"][0]["id"]')")
done
GW_X=${GW_NODES[0]}
cat >> "$GW_ROOT/openwarrant.toml" <<EOF

[go]
harness_argv = ["python3", "$GW_FIX"]
max_parallel = 2
EOF
git -C "$GW_ROOT" add -A >/dev/null 2>&1
git -C "$GW_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "eight items" >/dev/null 2>&1
GW_TWO="$GW_TMP/second"
git -C "$GW_ROOT" worktree add -q --detach "$GW_TWO" HEAD >/dev/null 2>&1
gww "$GW_ROOT" claim "$GW_X" --as someone >/dev/null 2>&1

gww "$GW_ROOT" --json evidence go > "$GW_TMP/one.json" 2>/dev/null &
GW_P1=$!
gww "$GW_TWO" --json evidence go > "$GW_TMP/two.json" 2>/dev/null &
GW_P2=$!
wait "$GW_P1"; wait "$GW_P2"
gww "$GW_TWO" --json evidence go > "$GW_TMP/three.json" 2>/dev/null
GW_SUM=$(python3 - "$GW_TMP/go.log" "$GW_X" "$GW_TMP/one.json" "$GW_TMP/two.json" "$GW_TMP/three.json" "${GW_NODES[@]}" <<'PY'
import json, re, sys
log, x, runs, nodes = sys.argv[1], sys.argv[2], sys.argv[3:6], sys.argv[6:]
starts = {}
for l in open(log):
    p = l.split()
    if p[0] == "start":
        starts[p[1]] = starts.get(p[1], 0) + 1
landed, held, third = {}, 0, None
for i, r in enumerate(runs):
    v = json.load(open(r))["result"]
    for row in v["landed"]:
        landed[row["node"]] = landed.get(row["node"], 0) + 1
    # The third run is alone: exactly X. The first two may also see each other's.
    held += bool(re.search(r"\b1 held by someone else" if i == 2 else r"[1-9][0-9]* held by someone else", v["stop"]["detail"]))
    if i == 2:
        third = len(v["landed"])
others = [n for n in nodes if n != x]
once = all(starts.get(n) == 1 and landed.get(n) == 1 for n in others)
print(once, starts.get(x, 0), landed.get(x, 0), held, sum(starts.values()))
PY
)
read -r GW_ONCE GW_XSTARTS GW_XLANDED GW_HELD GW_TOTAL <<<"$GW_SUM"
if [[ "$GW_ONCE" == "True" && "$GW_TOTAL" == "7" ]]; then
    gw_ok "two runs never double-run a node" "7 items, each started and landed exactly once across three runs in two worktrees"
else
    gw_fail "two runs never double-run a node" "once=$GW_ONCE, $GW_TOTAL starts (want 7)"
fi
if [[ "$GW_XSTARTS" == "0" && "$GW_XLANDED" == "0" && "${GW_HELD:-0}" == "3" ]]; then
    gw_ok "a node held elsewhere is not run" "$GW_X, claimed by someone: no run started it; $GW_HELD run(s) stopped naming it held"
else
    gw_fail "a node held elsewhere is not run" "$GW_XSTARTS starts, $GW_XLANDED landed, held said $GW_HELD"
fi

git -C "$GW_ROOT" worktree remove --force "$GW_TWO" >/dev/null 2>&1
command rm -rf "$GW_TMP"
unset GW_ROOT GW_TMP GW_WAR GW_FIX GW_NODES GW_X GW_TWO GW_P1 GW_P2 GW_SUM GW_ONCE GW_XSTARTS GW_XLANDED \
    GW_HELD GW_TOTAL
unset -f gw_ok gw_fail gww gw_field
