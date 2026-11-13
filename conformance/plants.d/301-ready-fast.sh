# shellcheck shell=bash
# The ready set, read fast, is the ready set (ticket::ready_rows_at).
#
# `war view ready` and `war next` ask only the tickets that can answer other
# than "ready" who waits on a person and whose scope is refused or conflicts,
# and read whose claim is whose once. OPENWARRANT_READY_REFERENCE=1 asks every
# ticket, as before. Over one corpus with epics and their parts, done tickets
# and done items, items that wait on other tickets, claims of several actors,
# a claim with declared paths, tickets that declare paths, and an Order handed
# to a person, the two answers are compared byte for byte:
# - `war view ready --json` and `war next --json` are identical either way,
#   and the ready set is not empty;
# - the gates the fast path skips for most tickets still close for the ones
#   they are for, in both readings: a ticket whose paths overlap another
#   holder's claim is not ready, and is ready again once that claim is
#   released; an Order handed to a person is not ready; a ticket whose paths
#   leave the repository is not ready; work in a roadmap phase that waits on
#   an unfinished phase is not ready, and is once that phase's work is done;
# - the comparison can fail: with one ready row taken out of one answer, the
#   same comparison says they differ.

echo "== the ready set, read fast, is the ready set =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID
RF_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$RF_T")
RF_ROOT="$RF_T/corpus"
RF_WAR="$REPO_ROOT/${WAR#./}"
rf_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rf_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
(cd "$REPO_ROOT" && tools/scale/synth-corpus.sh --war "$RF_WAR" --n 2 --resolved 0 --tickets 120 \
    --unsigned --out "$RF_ROOT") >"$RF_T/synth.log" 2>&1 \
    || { printf 'PLANT SETUP FAILED: synth-corpus.sh (%s)\n' "$(tail -1 "$RF_T/synth.log")" >&2; exit 9; }
rf_as() {
    local actor="$1"; shift
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_READY_REFERENCE OPENWARRANT_ACTOR="$actor" \
        "$RF_WAR" --root "$RF_ROOT" "$@" </dev/null
}
rf_id() { python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])'; }
# rf_ready <file-stem>: both readings of `view ready --json`, as the reader "reader".
rf_ready() {
    rf_as reader --json view ready > "$RF_T/$1.fast.json" 2>/dev/null
    OPENWARRANT_READY_REFERENCE=1 env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=reader \
        "$RF_WAR" --root "$RF_ROOT" --json view ready > "$RF_T/$1.plain.json" 2>/dev/null </dev/null
}
# rf_has <file> <ticket>: whether the ready rows of <file> name <ticket>.
rf_has() {
    python3 -c '
import json, sys
rows = json.load(open(sys.argv[1]))["result"]["ready"]
sys.exit(0 if any(r["ticket"] == sys.argv[2] for r in rows) else 1)' "$1" "$2"
}
rf_rows() { python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["result"]["ready"]))' "$1"; }

# Beside the generator's mix: declared paths, a holder with declared paths, an
# Order handed to a person, and a scope that leaves the repository.
mkdir -p "$RF_ROOT/src"; : > "$RF_ROOT/src/a.rs"
RF_PATHED=$(rf_as maker --json create "Touches the parser" --path "src/**" --item "Edit it" | rf_id)
RF_HOLDER=$(rf_as maker --json create "Holds the parser" --item "Hold it" | rf_id)
RF_ELSE=$(rf_as maker --json create "Touches the docs" --path "notes/**" --item "Write it" | rf_id)
RF_ORDER=$(rf_as maker --json create "Needs a person" --sign --item "Do the work" | rf_id)
RF_OUT=$(rf_as maker --json create "Leaves the repository" --item "Never ready" | rf_id)
[[ -n $RF_PATHED && -n $RF_HOLDER && -n $RF_ELSE && -n $RF_ORDER && -n $RF_OUT ]] \
    || { printf 'PLANT SETUP FAILED: the ready plant could not create its tickets\n' >&2; exit 9; }
sed -i 's|^alias: \(.*\)$|alias: \1\npaths:\n  - "/etc/**"|' "$(tk "$RF_ROOT/docs/tickets/$RF_OUT.md")"
# It is still a ticket the store reads: what keeps it from the ready set is its scope.
rf_as reader --json view tickets > "$RF_T/tickets.json" 2>/dev/null
python3 -c '
import json, sys
rows = json.load(open(sys.argv[1]))["result"]["tickets"]
sys.exit(0 if any(r.get("id") == sys.argv[2] for r in rows) else 1)' "$RF_T/tickets.json" "$RF_OUT" \
    || { printf 'PLANT SETUP FAILED: the ticket with a scope outside the repository is not read\n' >&2; exit 9; }
# Two roadmap phases, the second waiting on the first, and a ticket in each:
# work in a later phase waits on the phases before its own, by name.
mkdir -p "$RF_ROOT/docs/roadmap/atoms"
cat > "$RF_ROOT/docs/roadmap/roadmap.toml" <<'TOML'
schema = "oh.war/roadmap/v1"
uuid = "01a0cd3a-0a24-74cb-9a4b-5a83dfe420fa"
program = "SC"
prefix = "SC"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"

[[atoms]]
ordinal = 20
role = "phases"
path = "atoms/20-phases.yaml"
TOML
printf '# Intent\n\nTwo phases.\n' > "$RF_ROOT/docs/roadmap/atoms/10-intent.md"
cat > "$RF_ROOT/docs/roadmap/atoms/20-phases.yaml" <<'YAML'
schema: "oh.war/roadmap-phases/v1"

tiers: []

phases:
  - id: "SC-PHASE-1"
    title: "First"
    outcome: "the first thing"
    exit: "it is done"
    depends_on: []
    open: []

  - id: "SC-PHASE-2"
    title: "Second"
    outcome: "the second thing"
    exit: "it is done"
    depends_on: ["SC-PHASE-1"]
    open: []
YAML
RF_P1=$(rf_as maker --json create "Phase one work" --summary "In the first phase." --item "Do it" | rf_id)
RF_P2=$(rf_as maker --json create "Phase two work" --summary "In the second phase." --item "Do it" | rf_id)
[[ -n $RF_P1 && -n $RF_P2 ]] \
    || { printf 'PLANT SETUP FAILED: the ready plant could not create its phase tickets\n' >&2; exit 9; }
sed -i 's/^summary: .*/part_of: "SC-PHASE-1"\n&/' "$(tk "$RF_ROOT/docs/tickets/$RF_P1.md")"
sed -i 's/^summary: .*/part_of: "SC-PHASE-2"\n&/' "$(tk "$RF_ROOT/docs/tickets/$RF_P2.md")"
rf_as other claim "$RF_HOLDER" --paths "src/a.rs" >"$RF_T/claim.out" 2>&1; RF_CLAIM_RC=$?
rf_as worker claim "$RF_ORDER" >/dev/null 2>&1
rf_as worker done "$RF_ORDER" >"$RF_T/order.out" 2>&1

# --- the two readings agree ------------------------------------------------------------
rf_ready held
rf_as reader --json next > "$RF_T/next.fast.json" 2>/dev/null
OPENWARRANT_READY_REFERENCE=1 env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=reader \
    "$RF_WAR" --root "$RF_ROOT" --json next > "$RF_T/next.plain.json" 2>/dev/null </dev/null
RF_N=$(rf_rows "$RF_T/held.fast.json" 2>/dev/null || echo 0)
if (( RF_N > 20 )) && cmp -s "$RF_T/held.fast.json" "$RF_T/held.plain.json" \
    && [[ -s "$RF_T/next.fast.json" ]] && cmp -s "$RF_T/next.fast.json" "$RF_T/next.plain.json"; then
    rf_ok "fast and plain readings agree" "$RF_N ready rows, and war next: byte-identical with OPENWARRANT_READY_REFERENCE=1"
else
    rf_fail "fast and plain readings agree" "$RF_N rows; ready $(cmp "$RF_T/held.fast.json" "$RF_T/held.plain.json" 2>&1 | head -1); next $(cmp "$RF_T/next.fast.json" "$RF_T/next.plain.json" 2>&1 | head -1)"
fi

# --- the gates still close, in both readings ---------------------------------------------
RF_GATES=""
for reading in fast plain; do
    f="$RF_T/held.$reading.json"
    rf_has "$f" "$RF_PATHED" && RF_GATES+=" $reading:overlap-ready"
    rf_has "$f" "$RF_ELSE" || RF_GATES+=" $reading:disjoint-not-ready"
    rf_has "$f" "$RF_ORDER" && RF_GATES+=" $reading:order-ready"
    rf_has "$f" "$RF_OUT" && RF_GATES+=" $reading:outside-ready"
    rf_has "$f" "$RF_P1" || RF_GATES+=" $reading:phase-one-not-ready"
    rf_has "$f" "$RF_P2" && RF_GATES+=" $reading:later-phase-ready"
done
rf_as other done "$RF_HOLDER" --release >/dev/null 2>&1
rf_as worker claim "$RF_P1" >/dev/null 2>&1
rf_as worker done "$RF_P1" >/dev/null 2>&1
rf_ready released
for reading in fast plain; do
    rf_has "$RF_T/released.$reading.json" "$RF_PATHED" || RF_GATES+=" $reading:released-not-ready"
    rf_has "$RF_T/released.$reading.json" "$RF_P2" || RF_GATES+=" $reading:phase-finished-not-ready"
done
if [[ $RF_CLAIM_RC -eq 0 && -z $RF_GATES ]] && cmp -s "$RF_T/released.fast.json" "$RF_T/released.plain.json" \
    && grep -q 'waits on a person' "$RF_T/order.out"; then
    rf_ok "the skipped gates still close" "an overlapping claim, an Order handed over, a scope outside the repository and a later phase's work: not ready; the claim released and the earlier phase finished: ready"
else
    rf_fail "the skipped gates still close" "claim exit $RF_CLAIM_RC; wrong:${RF_GATES:- none}; $(head -c 160 "$RF_T/claim.out")"
fi

# --- the comparison can fail ------------------------------------------------------------
python3 - "$RF_T/held.fast.json" "$RF_T/cut.json" <<'PY'
import json, sys
v = json.load(open(sys.argv[1]))
v["result"]["ready"].pop()
json.dump(v, open(sys.argv[2], "w"))
PY
if ! cmp -s "$RF_T/held.plain.json" "$RF_T/cut.json" && (( $(rf_rows "$RF_T/cut.json") == RF_N - 1 )); then
    rf_ok "a missing row is a difference" "one of $RF_N rows taken out: the comparison says so"
else
    rf_fail "a missing row is a difference" "the cut answer compared equal"
fi
