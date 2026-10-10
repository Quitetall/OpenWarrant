# shellcheck shell=bash
# OW-WAR-0148 M15 — `war evidence go` over a 20-node graph with diamonds.
#
# One scratch program (GA). Four light Warrants, twenty items: A's a1; a2 and
# a3 after a1; a4 after a2 and a3 (a diamond); a5 after a4. B's b1; b2 after
# b1; b3 after b1 and A's a2; b4 after b2 and b3; b5 after b4. C's c1 after
# the whole of A; c2, c3 after c1; c4 after both; c5 after c4 and B's b5.
# D's d1; d2, d3 after d1; d4 after d2, d3 and B's b2; d5 after d4. And the
# program's own Warrant, whose agent stage STAGE-001 runs too (performed by
# `[perform] performer_argv`, the echo fixture) while STAGE-002 waits on a
# milestone no submission can complete. The performer of every item is the
# fixture conformance/fixtures/go/performer.py: no model.
#
# Accepted: the run lands every item and the open stage, and stops saying
# nothing is ready but the one stage waiting; no node started before every node it waits on was
# ticked (the fixture checks the checklist at its start, and the log's
# times agree with the graph's edges); at most three ran at once, and more
# than one did; every item's work is on war-go/integration, each tick saying
# where it landed, at `claimed` (nothing was checked); STAGE-002 still waits
# on M1.
# Refused, each by rule, with nothing claimed or started: a run with no
# harness (go.no-harness); several at once in the shared tree
# (go.no-containment); and `war evidence perform` still refuses
# [perform] max_concurrent above 1 (perform.no-containment).

echo "== war go: a 20-node graph with diamonds (M15) =="
GA_ROOT=$(scratch_corpus GA)
[[ -d "${GA_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
GA_TMP=$(mktemp -d)
GA_WAR="$REPO_ROOT/${WAR#./}"
GA_FIX="$REPO_ROOT/conformance/fixtures/go/performer.py"
GA_ECHO="$REPO_ROOT/conformance/fixtures/performer/echo-submission.sh"
ga_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ga_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
gaw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID GO_LOG="$GA_TMP/go.log" GO_SLEEP=1.5 \
    "$GA_WAR" --root "$GA_ROOT" "$@" </dev/null; }
ga_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
ga_new() { ga_field "$(gaw --json create "$@" 2>/dev/null)" 'v["result"]["id"] + " " + " ".join(i["id"] for i in v["result"]["items"])'; }
ga_add() { ga_field "$(gaw --json add "$@" 2>/dev/null)" 'v["result"]["item"]'; }
ga_commit() { git -C "$GA_ROOT" add -A >/dev/null 2>&1; git -C "$GA_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1; }
# Replace the [go] table (always the file's last) with the given lines.
ga_go() { python3 - "$GA_ROOT/openwarrant.toml" "$@" <<'PY'
import sys
p, lines = sys.argv[1], sys.argv[2:]
s = open(p).read()
i = s.find("\n[go]\n")
if i >= 0:
    s = s[:i + 1]
open(p, "w").write(s.rstrip("\n") + "\n\n[go]\n" + "".join(l + "\n" for l in lines))
PY
}

# ---------------------------------------------------------------- the graph --
read -r GA_A GA_a1 <<<"$(ga_new "Alpha" -i a1)"
GA_a2=$(ga_add "$GA_A" a2 --after "$GA_a1"); GA_a3=$(ga_add "$GA_A" a3 --after "$GA_a1")
GA_a4=$(ga_add "$GA_A" a4 --after "$GA_a2" --after "$GA_a3"); GA_a5=$(ga_add "$GA_A" a5 --after "$GA_a4")
read -r GA_B GA_b1 <<<"$(ga_new "Beta" -i b1)"
GA_b2=$(ga_add "$GA_B" b2 --after "$GA_b1"); GA_b3=$(ga_add "$GA_B" b3 --after "$GA_b1" --after "$GA_A/$GA_a2")
GA_b4=$(ga_add "$GA_B" b4 --after "$GA_b2" --after "$GA_b3"); GA_b5=$(ga_add "$GA_B" b5 --after "$GA_b4")
read -r GA_C <<<"$(ga_new "Gamma")"
GA_c1=$(ga_add "$GA_C" c1 --after "$GA_A"); GA_c2=$(ga_add "$GA_C" c2 --after "$GA_c1"); GA_c3=$(ga_add "$GA_C" c3 --after "$GA_c1")
GA_c4=$(ga_add "$GA_C" c4 --after "$GA_c2" --after "$GA_c3"); GA_c5=$(ga_add "$GA_C" c5 --after "$GA_c4" --after "$GA_B/$GA_b5")
read -r GA_D GA_d1 <<<"$(ga_new "Delta" -i d1)"
GA_d2=$(ga_add "$GA_D" d2 --after "$GA_d1"); GA_d3=$(ga_add "$GA_D" d3 --after "$GA_d1")
GA_d4=$(ga_add "$GA_D" d4 --after "$GA_d2" --after "$GA_d3" --after "$GA_B/$GA_b2"); GA_d5=$(ga_add "$GA_D" d5 --after "$GA_d4")
# The program's stages say what runs them, and the stage performer is the echo fixture.
sed -i 's/^    executor_kind: "agent"$/    executor_kind: "agent"\n    executor_ref: "agent:\/\/fixture"/' \
    "$GA_ROOT/docs/warrants/GA-WAR-0001/atoms/45-milestones.yaml"
sed -i "s|^performer_argv = \[\]|performer_argv = [\"bash\", \"$GA_ECHO\"]|" "$GA_ROOT/openwarrant.toml"
gaw admin compile >/dev/null 2>&1
ga_commit "twenty items and a stage"
GA_EDGES=$(gaw --json plan frontier --all 2>/dev/null)
GA_ITEMS=$(ga_field "$GA_EDGES" 'sum(1 for n in v["result"]["nodes"] if n["kind"] == "item")')

# ---------------------------------------------------------------- refusals --
# No harness: refused before anything is claimed.
ga_go 'max_parallel = 3'
GA_OUT=$(gaw --json evidence go --prototype 2>/dev/null); GA_RC=$?
GA_RULES=$(ga_field "$GA_OUT" '",".join(d["rule"] for d in v["diagnostics"] if d["severity"] == "error")')
GA_DISPATCHED=$(cat "$GA_ROOT"/docs/tickets/*/journal.jsonl | grep -c '"type":"go.dispatched"')
if [[ $GA_RC -eq 2 && "$GA_RULES" == "go.no-harness" && "$GA_DISPATCHED" == "0" && ! -e "$GA_TMP/go.log" ]]; then
    ga_ok "no harness is refused" "go.no-harness, exit 2; nothing claimed or started"
else
    ga_fail "no harness is refused" "exit $GA_RC, rules '$GA_RULES', $GA_DISPATCHED dispatched"
fi
# Several at once in one shared tree: refused; `war evidence perform` keeps its own refusal.
ga_go "harness_argv = [\"python3\", \"$GA_FIX\"]" 'max_parallel = 3' 'isolation = "shared"'
GA_OUT=$(gaw --json evidence go --prototype 2>/dev/null); GA_RC=$?
GA_RULES=$(ga_field "$GA_OUT" '",".join(d["rule"] for d in v["diagnostics"] if d["severity"] == "error")')
sed -i 's/^max_concurrent = 0$/max_concurrent = 2/' "$GA_ROOT/openwarrant.toml"
GA_PERF=$(gaw --json evidence perform GA-WAR-0001 STAGE-001 --prototype 2>/dev/null); GA_PRC=$?
GA_PRULES=$(ga_field "$GA_PERF" '",".join(d["rule"] for d in v["diagnostics"] if d["severity"] == "error")')
sed -i 's/^max_concurrent = 2$/max_concurrent = 0/' "$GA_ROOT/openwarrant.toml"
GA_DISPATCHED=$(cat "$GA_ROOT"/docs/tickets/*/journal.jsonl | grep -c '"type":"go.dispatched"')
if [[ $GA_RC -eq 2 && "$GA_RULES" == "go.no-containment" && $GA_PRC -eq 2 \
    && "$GA_PRULES" == "perform.no-containment" && "$GA_DISPATCHED" == "0" && ! -e "$GA_TMP/go.log" ]]; then
    ga_ok "several at once in one tree refused" "go.no-containment; perform.no-containment still; nothing started"
else
    ga_fail "several at once in one tree refused" "go exit $GA_RC '$GA_RULES'; perform exit $GA_PRC '$GA_PRULES'; $GA_DISPATCHED dispatched"
fi

# ---------------------------------------------------------------- the run --
ga_go "harness_argv = [\"python3\", \"$GA_FIX\"]" 'max_parallel = 3'
GA_OUT=$(gaw --json evidence go --prototype 2>/dev/null); GA_RC=$?
GA_STOP=$(ga_field "$GA_OUT" 'v["result"]["stop"]["reason"] + ": " + v["result"]["stop"]["detail"]')
GA_LANDED=$(ga_field "$GA_OUT" 'len(v["result"]["landed"])')
GA_TICKED=$(cat "$GA_ROOT"/docs/tickets/*/atoms/15-checklist.md | grep -c '^- \[x\] .* war go: landed [0-9a-f]* on war-go/integration$')
GA_STAGES=$(gaw --json plan frontier 2>/dev/null)
GA_S1=$(ga_field "$GA_STAGES" '[r["state"] for r in v["result"]["rows"] if r["stage"] == "STAGE-001"][0]')
GA_S2=$(ga_field "$GA_STAGES" '[r["state"] for r in v["result"]["rows"] if r["stage"] == "STAGE-002"][0]')
if [[ $GA_RC -eq 0 && "$GA_STOP" == "nothing-ready: nothing is ready: 1 waiting on other work, 0 blocked, 0 held by someone else, 0 not started by this run" && "$GA_ITEMS" == "20" && "$GA_LANDED" == "21" \
    && "$GA_TICKED" == "20" && "$GA_S1" == "done" && "$GA_S2" == "blocked" ]]; then
    ga_ok "a 20-node graph lands" "20 items and STAGE-001 landed; STAGE-002 still waits on M1, and the stop says so"
else
    ga_fail "a 20-node graph lands" "exit $GA_RC, stop '$GA_STOP', $GA_ITEMS items, $GA_LANDED landed, $GA_TICKED ticked, stages $GA_S1/$GA_S2"
fi

# Order: the fixture's own check at each start, and the log's times against the graph's edges.
GA_ORDER=$(python3 - "$GA_TMP/go.log" "$GA_EDGES" <<'PY'
import json, sys
log = [l.split() for l in open(sys.argv[1])]
graph = json.loads(sys.argv[2])["result"]
start, end = {}, {}
for l in log:
    if l[0] == "start":
        start.setdefault(l[1], int(l[3]))
    elif l[0] == "end":
        end[l[1]] = int(l[3])
items = {n["id"] for n in graph["nodes"] if n["kind"] == "item"}
deps = {}
for e in graph["edges"]:
    deps.setdefault(e["from"], set()).add(e["to"])
bad = []
for n in items:
    for d in deps.get(n, ()):
        made = [d] if d in items else sorted(deps.get(d, ()))   # a Warrant: its items
        for m in made:
            if m not in end or n not in start or end[m] > start[n]:
                bad.append(f"{n} before {m}")
violations = sum(1 for l in log if l[0] == "VIOLATION")
print(f"{len(bad)} {violations} {len(start)} {sum(len(deps.get(n, ())) for n in items)}")
PY
)
read -r GA_BAD GA_VIOL GA_STARTED GA_NEDGES <<<"$GA_ORDER"
if [[ "$GA_BAD" == "0" && "$GA_VIOL" == "0" && "$GA_STARTED" == "20" && "${GA_NEDGES:-0}" -ge 20 ]]; then
    ga_ok "no node starts before its deps" "$GA_NEDGES edges held; the fixture saw every dependency ticked"
else
    ga_fail "no node starts before its deps" "$GA_BAD out of order, $GA_VIOL violations, $GA_STARTED started, $GA_NEDGES edges"
fi
# The refusal side of the same reader: a log with one start moved before its dependency's end is caught.
python3 - "$GA_TMP/go.log" "$GA_A/$GA_a1" "$GA_A/$GA_a2" > "$GA_TMP/planted.log" <<'PY'
import sys
lines = [l.split() for l in open(sys.argv[1])]
first = {l[1]: int(l[3]) for l in lines if l[0] == "end" and l[1] == sys.argv[2]}
for l in lines:
    if l[0] == "start" and l[1] == sys.argv[3]:
        l[3] = str(first[sys.argv[2]] - 1)
    print(" ".join(l))
PY
GA_ORDER=$(python3 - "$GA_TMP/planted.log" "$GA_EDGES" <<'PY'
import json, sys
log = [l.split() for l in open(sys.argv[1])]
graph = json.loads(sys.argv[2])["result"]
start = {l[1]: int(l[3]) for l in log if l[0] == "start"}
end = {l[1]: int(l[3]) for l in log if l[0] == "end"}
items = {n["id"] for n in graph["nodes"] if n["kind"] == "item"}
bad = [(e["from"], e["to"]) for e in graph["edges"]
       if e["from"] in items and e["to"] in items and end[e["to"]] > start[e["from"]]]
print(len(bad))
PY
)
if [[ "$GA_ORDER" == "1" ]]; then
    ga_ok "a planted early start is caught" "the reader names the one edge moved"
else
    ga_fail "a planted early start is caught" "the reader found $GA_ORDER out of order (want 1)"
fi

# Concurrency, as the fixture counted it under its lock.
GA_PEAK=$(grep -o 'running=[0-9]*' "$GA_TMP/go.log" | cut -d= -f2 | sort -n | tail -1)
GA_RPEAK=$(ga_field "$GA_OUT" 'v["result"]["peak_parallel"]')
if [[ "${GA_PEAK:-0}" -le 3 && "${GA_PEAK:-0}" -ge 2 && "${GA_RPEAK:-0}" -le 3 ]]; then
    ga_ok "concurrency never exceeds the cap" "the fixture saw at most $GA_PEAK at once (cap 3); the run says $GA_RPEAK"
else
    ga_fail "concurrency never exceeds the cap" "fixture peak '$GA_PEAK', run peak '$GA_RPEAK', cap 3"
fi

# The work is on the integration branch; the checkout's own branch did not move.
GA_WORK=$(git -C "$GA_ROOT" ls-tree -r --name-only war-go/integration -- work | grep -c '^work/t-')
GA_HEAD_WORK=$(git -C "$GA_ROOT" ls-tree -r --name-only HEAD -- work | grep -c '^work/' )
GA_TREES=$(git -C "$GA_ROOT" worktree list | wc -l)
if [[ "$GA_WORK" == "20" && "$GA_HEAD_WORK" == "0" && "$GA_TREES" == "1" ]]; then
    ga_ok "work lands on war-go/integration" "20 work files there, none on HEAD; every node's worktree removed"
else
    ga_fail "work lands on war-go/integration" "$GA_WORK on integration, $GA_HEAD_WORK on HEAD, $GA_TREES worktrees"
fi

command rm -rf "$GA_TMP"
unset GA_ROOT GA_TMP GA_WAR GA_FIX GA_ECHO GA_OUT GA_RC GA_RULES GA_DISPATCHED GA_PERF GA_PRC GA_PRULES \
    GA_STOP GA_LANDED GA_TICKED GA_STAGES GA_S1 GA_S2 GA_EDGES GA_ITEMS GA_ORDER GA_BAD GA_VIOL GA_STARTED \
    GA_NEDGES GA_PEAK GA_RPEAK GA_WORK GA_HEAD_WORK GA_TREES \
    GA_A GA_a1 GA_a2 GA_a3 GA_a4 GA_a5 GA_B GA_b1 GA_b2 GA_b3 GA_b4 GA_b5 GA_C GA_c1 GA_c2 GA_c3 GA_c4 GA_c5 \
    GA_D GA_d1 GA_d2 GA_d3 GA_d4 GA_d5
unset -f ga_ok ga_fail gaw ga_field ga_new ga_add ga_commit ga_go
