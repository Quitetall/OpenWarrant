# shellcheck shell=bash
# OW-WAR-0148 M15 — the schedule: estimates learned from the journal, the
# critical path, deadlines, and cycles.
#
# One scratch program (GE). Journal history is planted in two done light
# Warrants: a bug labelled ui (claim to done 600s and 1200s) and a feature
# (3600s, and one tick with no claim before it). Open work: N1 (bug, ui; one
# item), N2 (feature; n2a, then n2b after it), N3 (chore; one item), N4 (one
# item after N3's).
#
# Accepted: before the history, an estimate is the prior (30m, `prior`);
# after it, N1's is the median of its type and label (900s, `type+label`,
# n=2), N2's of its type (3600s, `type`), N3's of all (1200s, `all`, n=3);
# the critical path is N2's two items (7200s), and it starts first. A due
# date on N4 (not ready: it waits on N3) makes N3 start first, and N4 itself
# is in no ready order: a date raises priority, never reorders what waits.
# Refused: a tick with no claim before it is no sample (3 samples, not 4);
# `war edit --due 2026-02-30` (ticket.due-invalid), nothing written; and a
# cycle across two Warrants (graph.cycle, naming both), for which `war
# evidence go` starts nothing and `war plan estimate` answers not ready, until
# the edge is taken out again.

echo "== war go: estimates, the critical path, deadlines, cycles (M15) =="
GE_ROOT=$(scratch_corpus GE)
[[ -d "${GE_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
GE_WAR="$REPO_ROOT/${WAR#./}"
ge_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ge_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
gew() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$GE_WAR" --root "$GE_ROOT" "$@" </dev/null; }
ge_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
ge_new() { ge_field "$(gew --json create "$@" 2>/dev/null)" 'v["result"]["id"] + " " + " ".join(i["id"] for i in v["result"]["items"])'; }
ge_est() { ge_field "$GE_EST" "[(r['estimate']['seconds'], r['estimate']['source'], r['estimate']['samples']) for r in v['result']['rows'] if r['id'] == '$1'][0]"; }
# plant <ticket> <event> <target> <time>: one journal line, as `war` writes one.
ge_plant() { python3 - "$GE_ROOT/docs/tickets/$1" "$2" "$3" "$4" <<'PY'
import hashlib, json, sys, tomllib, uuid
d, kind, target, at = sys.argv[1:]
m = tomllib.load(open(f"{d}/manifest.toml", "rb"))
payload = json.dumps({"target": target, "since": at} if kind.endswith("claimed") else {"target": target, "on": at[:10]})
key = hashlib.sha256(f"{m['uuid']}\n{kind}\n{payload}".encode()).hexdigest()
ev = {"v": 1, "id": str(uuid.uuid4()), "warrant_uuid": m["uuid"], "type": kind, "class": "draft_history",
      "actor_ref": "planted", "occurred_at": at, "payload": payload, "idempotency_key": key}
open(f"{d}/journal.jsonl", "a").write(json.dumps(ev, separators=(",", ":")) + "\n")
PY
}

read -r GE_N1 GE_n1 <<<"$(ge_new "Open bug" --type bug -l ui -i n1)"
read -r GE_N2 GE_n2a <<<"$(ge_new "Open feature" --type feature -i n2a)"
GE_n2b=$(ge_field "$(gew --json add "$GE_N2" n2b --after "$GE_n2a" 2>/dev/null)" 'v["result"]["item"]')
read -r GE_N3 GE_n3 <<<"$(ge_new "Open chore" --type chore -i n3)"
read -r GE_N4 GE_n4 <<<"$(ge_new "After the chore" -i n4)"
gew add "$GE_N4" n4b --after "$GE_N3/$GE_n3" >/dev/null 2>&1
GE_n4=$(sed -n 's/^- \[ \] n4b (\(i-[0-9a-f]*\), after .*)$/\1/p' "$GE_ROOT/docs/tickets/$GE_N4/atoms/15-checklist.md")
# N4's own first item is out of the way: ticked by hand (the file is the state).
sed -i 's/^- \[ \] n4 (/- [x] n4 (/' "$GE_ROOT/docs/tickets/$GE_N4/atoms/15-checklist.md"

GE_EST=$(gew --json plan estimate 2>/dev/null)
GE_PRIOR=$(ge_est "$GE_N1/$GE_n1")
if [[ "$GE_PRIOR" == "(1800, 'prior', 0)" ]]; then
    ge_ok "no history: the prior" "N1 is 30m, from the prior, on 0 samples"
else
    ge_fail "no history: the prior" "N1 is $GE_PRIOR"
fi

# The planted history.
read -r GE_H1 GE_h1 GE_h2 <<<"$(ge_new "Done bug" --type bug -l ui -i h1 -i h2)"
read -r GE_H2 GE_f1 GE_f2 <<<"$(ge_new "Done feature" --type feature -i f1 -i f2)"
ge_plant "$GE_H1" ticket.claimed "$GE_H1/$GE_h1" 2026-10-01T10:00:00Z
ge_plant "$GE_H1" ticket.item_done "$GE_H1/$GE_h1" 2026-10-01T10:10:00Z
ge_plant "$GE_H1" ticket.claimed "$GE_H1/$GE_h2" 2026-10-01T10:20:00Z
ge_plant "$GE_H1" ticket.item_done "$GE_H1/$GE_h2" 2026-10-01T10:40:00Z
ge_plant "$GE_H2" ticket.claimed "$GE_H2/$GE_f1" 2026-10-02T09:00:00Z
ge_plant "$GE_H2" ticket.item_done "$GE_H2/$GE_f1" 2026-10-02T10:00:00Z
ge_plant "$GE_H2" ticket.item_done "$GE_H2/$GE_f2" 2026-10-02T11:00:00Z
sed -i 's/^- \[ \] /- [x] /' "$GE_ROOT/docs/tickets/$GE_H1/atoms/15-checklist.md" "$GE_ROOT/docs/tickets/$GE_H2/atoms/15-checklist.md"

GE_EST=$(gew --json plan estimate 2>/dev/null)
GE_E1=$(ge_est "$GE_N1/$GE_n1"); GE_E2=$(ge_est "$GE_N2/$GE_n2a"); GE_E3=$(ge_est "$GE_N3/$GE_n3")
GE_SAMPLES=$(ge_field "$GE_EST" '(v["result"]["history"]["samples"], v["result"]["history"]["unclaimed_ticks"])')
if [[ "$GE_E1" == "(900, 'type+label', 2)" && "$GE_E2" == "(3600, 'type', 1)" && "$GE_E3" == "(1200, 'all', 3)" ]]; then
    ge_ok "estimates come from the journal" "bug+ui 15m (n=2), feature 1h (n=1), chore 20m from all (n=3)"
else
    ge_fail "estimates come from the journal" "N1 $GE_E1, N2 $GE_E2, N3 $GE_E3"
fi
if [[ "$GE_SAMPLES" == "(3, 1)" ]]; then
    ge_ok "a tick with no claim is no sample" "3 samples; 1 tick with no claim before it, counted and left out"
else
    ge_fail "a tick with no claim is no sample" "samples, unclaimed: $GE_SAMPLES"
fi
GE_CP=$(ge_field "$GE_EST" '" ".join(v["result"]["critical_path"]) + " " + str(v["result"]["critical_secs"])')
GE_ORDER=$(ge_field "$(gew --json evidence go --dry-run 2>/dev/null)" '" ".join(v["result"]["order"])')
if [[ "$GE_CP" == "$GE_N2/$GE_n2a $GE_N2/$GE_n2b 7200" && "${GE_ORDER%% *}" == "$GE_N2/$GE_n2a" ]]; then
    ge_ok "the critical path starts first" "$GE_N2: two items, 2h; first in the order"
else
    ge_fail "the critical path starts first" "path '$GE_CP', order '$GE_ORDER'"
fi

# A deadline on work that waits: what it waits on goes first; it does not.
GE_BEFORE=$(cat "$GE_ROOT/docs/tickets/$GE_N1/manifest.toml")
GE_BAD=$(gew --json edit "$GE_N1" --due 2026-02-30 2>/dev/null); GE_BADRC=$?
GE_BADRULE=$(ge_field "$GE_BAD" '",".join(d["rule"] for d in v["diagnostics"] if d["severity"] == "error")')
gew edit "$GE_N4" --due 2026-10-20 >/dev/null 2>&1
GE_ORDER=$(ge_field "$(gew --json evidence go --dry-run 2>/dev/null)" '" ".join(v["result"]["order"])')
GE_DUE=$(grep -c '^due = "2026-10-20"$' "$GE_ROOT/docs/tickets/$GE_N4/manifest.toml")
if [[ "${GE_ORDER%% *}" == "$GE_N3/$GE_n3" && "$GE_ORDER" != *"$GE_N4/$GE_n4"* && "$GE_DUE" == "1" ]]; then
    ge_ok "a deadline raises what it waits on" "$GE_N4 due 2026-10-20: $GE_N3/$GE_n3 starts first; $GE_N4 is not ready, so in no order"
else
    ge_fail "a deadline raises what it waits on" "order '$GE_ORDER', due lines $GE_DUE"
fi
if [[ $GE_BADRC -eq 2 && "$GE_BADRULE" == "ticket.due-invalid" \
    && "$(cat "$GE_ROOT/docs/tickets/$GE_N1/manifest.toml")" == "$GE_BEFORE" ]]; then
    ge_ok "a date that is no date is refused" "--due 2026-02-30: ticket.due-invalid, the manifest unchanged"
else
    ge_fail "a date that is no date is refused" "exit $GE_BADRC, '$GE_BADRULE'"
fi

# A cycle across two Warrants, made by hand: refused by name, nothing started.
read -r GE_C1 GE_c1 <<<"$(ge_new "Cycle one" -i c1)"
read -r GE_C2 GE_c2 <<<"$(ge_new "Cycle two" -i c2)"
gew add "$GE_C2" c3 --after "$GE_C1/$GE_c1" >/dev/null 2>&1
GE_c3=$(sed -n 's/^- \[ \] c3 (\(i-[0-9a-f]*\), after .*)$/\1/p' "$GE_ROOT/docs/tickets/$GE_C2/atoms/15-checklist.md")
cat >> "$GE_ROOT/openwarrant.toml" <<'EOF'

[go]
harness_argv = ["false"]
EOF
sed -i "s|^- \[ \] c1 ($GE_c1)\$|- [ ] c1 ($GE_c1, after $GE_C2/$GE_c3)|" "$GE_ROOT/docs/tickets/$GE_C1/atoms/15-checklist.md"
GE_GO=$(gew --json evidence go 2>/dev/null); GE_GORC=$?
GE_GORULE=$(ge_field "$GE_GO" '",".join(d["rule"] + ":" + d["message"].split(" wait on")[0] for d in v["diagnostics"] if d["severity"] == "error")')
GE_ESTRC=$(gew --json plan estimate >/dev/null 2>&1; echo $?)
GE_DISPATCHED=$(cat "$GE_ROOT"/docs/tickets/*/journal.jsonl | grep -c '"type":"go.dispatched"')
GE_WANT="graph.cycle:$(printf '%s\n' "$GE_C1/$GE_c1" "$GE_C2/$GE_c3" | sort | paste -sd, - | sed 's/,/, /')"
sed -i "s|^- \[ \] c1 ($GE_c1, after $GE_C2/$GE_c3)\$|- [ ] c1 ($GE_c1)|" "$GE_ROOT/docs/tickets/$GE_C1/atoms/15-checklist.md"
GE_AFTER=$(gew --json evidence go --dry-run >/dev/null 2>&1; echo $?)
if [[ $GE_GORC -eq 2 && "$GE_GORULE" == "$GE_WANT" && "$GE_ESTRC" == "2" && "$GE_DISPATCHED" == "0" && "$GE_AFTER" == "0" ]]; then
    ge_ok "a cycle is refused by name" "graph.cycle names both; go started nothing, estimate not ready; without the edge, accepted"
else
    ge_fail "a cycle is refused by name" "go exit $GE_GORC '$GE_GORULE' (want '$GE_WANT'), estimate $GE_ESTRC, $GE_DISPATCHED dispatched, after $GE_AFTER"
fi

unset GE_ROOT GE_WAR GE_N1 GE_n1 GE_N2 GE_n2a GE_n2b GE_N3 GE_n3 GE_N4 GE_n4 GE_EST GE_PRIOR GE_H1 GE_h1 GE_h2 \
    GE_H2 GE_f1 GE_f2 GE_E1 GE_E2 GE_E3 GE_SAMPLES GE_CP GE_ORDER GE_BEFORE GE_BAD GE_BADRC GE_BADRULE GE_DUE \
    GE_C1 GE_c1 GE_C2 GE_c2 GE_c3 GE_GO GE_GORC GE_GORULE GE_ESTRC GE_DISPATCHED GE_WANT GE_AFTER
unset -f ge_ok ge_fail gew ge_field ge_new ge_est ge_plant
