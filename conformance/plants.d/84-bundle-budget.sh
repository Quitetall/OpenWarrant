# shellcheck shell=bash
# t-9f7e — a verification bundle is bounded, and says what it did not carry.
#
# OW-WAR-0112's one bundle was 241k tokens and the blind verifier timed out
# on it. A Warrant whose bundle exceeds `[verify] max_bundle_tokens` is split
# into one bundle per obligation, each carrying what that obligation names,
# excerpted to fit, with the whole file's digest. A scratch program with a
# 20,000-line deliverable and a small budget: every bundle fits, the named
# line is carried, and what is absent or cut is said — never dropped.

echo "== bounded verification bundles (t-9f7e) =="
PLANT_ROOT=$(scratch_corpus BB)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
BB_W="$PLANT_ROOT/docs/warrants/BB-WAR-0001"
BB_A="$BB_W/atoms/60-assurance.md"
bb_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
bb_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# The fixture: a large deliverable naming a marker on line 15000, a second
# deliverable that is not on disk, and OBL-001's evidence naming the marker,
# a path that exists nowhere and a term that appears nowhere.
mkdir -p "$PLANT_ROOT/src"
python3 - "$PLANT_ROOT/src/big.txt" <<'PY'
import sys
with open(sys.argv[1], "w") as f:
    for i in range(1, 20001):
        f.write("bundle-marker-term: the named line\n" if i == 15000 else "filler line %d of a large deliverable\n" % i)
PY
cat > "$BB_W/deliverables.toml" <<'TOML'
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "a large fixture"
kind = "file"
target_ref = "src/big.txt"
required = true
obligation_refs = ["OBL-001"]

[[deliverable]]
id = "D-002"
title = "a deliverable that is not on disk"
kind = "file"
target_ref = "src/gone.txt"
required = true
obligation_refs = ["OBL-002"]
TOML
python3 - "$BB_A" <<'PY'
import re, sys
p = sys.argv[1]; t = open(p).read()
t = t.replace("`war check` reports no `sas.unrecorded`.",
    "`war check` reports no `sas.unrecorded`; `src/big.txt` holds `bundle-marker-term`; "
    "`src/nowhere.rs` holds `never-said-anywhere`.", 1)
open(p, "w").write(t)
PY
grep -q 'never-said-anywhere' "$BB_A" || { printf 'PLANT SETUP FAILED: could not name the evidence in %s\n' "$BB_A" >&2; exit 9; }
bb_budget() { # tokens → set `[verify] max_bundle_tokens` in the scratch
    sed -i "s/^max_bundle_tokens = .*/max_bundle_tokens = $1/" "$PLANT_ROOT/openwarrant.toml"
    grep -qx "max_bundle_tokens = $1" "$PLANT_ROOT/openwarrant.toml" \
        || { printf 'PLANT SETUP FAILED: could not set max_bundle_tokens\n' >&2; exit 9; }
}
bb_bundle() { # → the report; the bundles are left in verifications/
    command rm -f "$BB_W"/verifications/bundle-*.json
    "$WAR" --root "$PLANT_ROOT" verify BB-WAR-0001 --performer claude --bundle 2>&1
}
bb_q() { # python over `bundles` (obligation id → bundle) → printed value
    python3 -c "
import glob, json, os, sys
bundles = {}
sizes = {}
for p in sorted(glob.glob(sys.argv[1] + '/verifications/bundle-*.json')):
    b = json.load(open(p))
    for o in b['request']['obligations']:
        bundles[o['id']] = b
        sizes[o['id']] = os.path.getsize(p)
print($1)" "$BB_W"
}
bb_budget 6000
BB_OUT=$(bb_bundle)
BB_N=$(ls "$BB_W"/verifications/bundle-*.json 2>/dev/null | wc -l)
BB_SHA=$(sha256sum "$PLANT_ROOT/src/big.txt" | cut -c1-64)
BB_LINE=$(sed -n 15000p "$PLANT_ROOT/src/big.txt")

# Accepting: split, every bundle within the budget, the named line carried.
if [[ "$BB_N" -eq 2 ]] \
    && [[ "$(bb_q "all(b['scope'] == 'obligation' and b['estimated_tokens'] <= 6000 and not b['over_budget'] for b in bundles.values())")" == "True" ]] \
    && [[ "$(bb_q "all(s <= 6000 * 4 + 64 for s in sizes.values())")" == "True" ]]; then
    bb_ok "a large Warrant is split to fit" "$BB_N obligation bundles, each <= 6000 tokens"
else
    bb_fail "a large Warrant is split to fit" "$BB_N bundle(s); $(bb_q "[(k, b['estimated_tokens'], sizes[k]) for k, b in bundles.items()]" 2>&1 | head -c 200)"
fi
BB_D="next(d for d in bundles['OBL-001']['deliverables'] if d['id'] == 'D-001')"
if [[ "$(python3 -c "
import glob, json, sys
for p in glob.glob(sys.argv[1] + '/verifications/bundle-*.json'):
    b = json.load(open(p))
    if b['request']['obligations'][0]['id'] != 'OBL-001': continue
    d = next(d for d in b['deliverables'] if d['id'] == 'D-001')
    print(any(e['start_line'] <= 15000 <= e['end_line'] and e['text'].splitlines()[15000 - e['start_line']] == sys.argv[2] for e in d['excerpts']))
" "$BB_W" "$BB_LINE")" == "True" ]]; then
    bb_ok "the named line is carried" "line 15000 of src/big.txt, numbered, byte-exact"
else
    bb_fail "the named line is carried" "$(bb_q "[(e['start_line'], e['end_line']) for e in $BB_D['excerpts']]" 2>&1 | head -c 200)"
fi
if [[ "$(bb_q "$BB_D['truncated'] and $BB_D['sha256'] == '$BB_SHA' and $BB_D['lines'] == 20000 and 'text' not in $BB_D and sum(e['end_line'] - e['start_line'] + 1 for e in $BB_D['excerpts']) < 20000")" == "True" ]]; then
    bb_ok "truncation is marked" "truncated, whole-file sha256 $BB_SHA, 20000 lines, fewer carried"
else
    bb_fail "truncation is marked" "$(bb_q "{k: v for k, v in $BB_D.items() if k not in ('excerpts', 'text')}" 2>&1 | head -c 300)"
fi
if [[ "$(bb_q "[t['shown_in'] for t in bundles['OBL-001']['obligation_evidence'][0]['terms'] if t['term'] == 'bundle-marker-term'] == [['D-001']]")" == "True" ]]; then
    bb_ok "the bundle says where a term is" "bundle-marker-term shown in D-001"
else
    bb_fail "the bundle says where a term is" "$(bb_q "bundles['OBL-001']['obligation_evidence'][0]['terms']" 2>&1 | head -c 300)"
fi
# Deterministic: two runs, the same files.
BB_FIRST=$(ls "$BB_W"/verifications/bundle-*.json | xargs -n1 basename | sort | tr '\n' ' ')
"$WAR" --root "$PLANT_ROOT" verify BB-WAR-0001 --performer claude --bundle >/dev/null 2>&1
BB_SECOND=$(ls "$BB_W"/verifications/bundle-*.json | xargs -n1 basename | sort | tr '\n' ' ')
if [[ -n "$BB_FIRST" && "$BB_FIRST" == "$BB_SECOND" ]]; then
    bb_ok "split bundles are deterministic" "two runs, the same $BB_N digests"
else
    bb_fail "split bundles are deterministic" "[$BB_FIRST] then [$BB_SECOND]"
fi

# Refusing: what the obligation names and the bundle cannot carry is said.
if [[ "$(bb_q "[(p['carried'], 'absent' in p) for p in bundles['OBL-001']['obligation_evidence'][0]['named_paths'] if p['term'] == 'src/nowhere.rs'] == [([], True)]")" == "True" ]] \
    && [[ "$(bb_q "[(t['shown_in'], t['cut_from']) for t in bundles['OBL-001']['obligation_evidence'][0]['terms'] if t['term'] == 'never-said-anywhere'] == [([], [])]")" == "True" ]] \
    && grep -qE '\[OBL-001\].* [1-9][0-9]* named path\(s\) not carried' <<<"$BB_OUT"; then
    bb_ok "absent named evidence is said" "src/nowhere.rs absent; never-said-anywhere shown nowhere"
else
    bb_fail "absent named evidence is said" "$(bb_q "bundles['OBL-001']['obligation_evidence']" 2>&1 | head -c 300)"
fi
BB_G="next(d for d in bundles['OBL-002']['deliverables'] if d['id'] == 'D-002')"
if [[ "$(bb_q "$BB_G['present'] is False and 'text' not in $BB_G and 'unreadable' in $BB_G['error']")" == "True" ]]; then
    bb_ok "a missing deliverable is not empty" "present false, error, no text"
else
    bb_fail "a missing deliverable is not empty" "$(bb_q "$BB_G" 2>&1 | head -c 300)"
fi
# Refusing: a budget nothing can meet is written, flagged and warned — not
# silently exceeded.
bb_budget 100
BB_OUT=$(bb_bundle)
if grep -q 'verify.bundle-over-budget' <<<"$BB_OUT" \
    && [[ "$(bb_q "all(b['over_budget'] for b in bundles.values())")" == "True" ]]; then
    bb_ok "an unmeetable budget is said" "verify.bundle-over-budget; over_budget true"
else
    bb_fail "an unmeetable budget is said" "$(head -3 <<<"$BB_OUT")"
fi
bb_budget 6000

# Refusing: a verifier answering an obligation its bundle did not carry is
# not recorded; the fixture that answers only what it was shown is.
# Same v2 fixture protocol, but deliberately answer both obligations even
# when a split packet carries only one. Refuse for scope, not legacy format.
command cp conformance/fixtures/verifier/establishes-all.sh "$PLANT_ROOT/answers-all.sh"
python3 - "$PLANT_ROOT/answers-all.sh" <<'PY'
import sys
p=sys.argv[1];s=open(p).read();old='for o in b["request"]["obligations"]:'
assert old in s
open(p,"w").write(s.replace(old,'for o in [{"id": "OBL-001"}, {"id": "OBL-002"}]:',1))
PY
sed -i "s|^verifier_argv = .*|verifier_argv = [\"bash\", \"$PLANT_ROOT/answers-all.sh\"]|" "$PLANT_ROOT/openwarrant.toml"
BB_RUN=$("$WAR" --root "$PLANT_ROOT" verify BB-WAR-0001 --performer claude --run 2>&1); BB_STATUS=$?
BB_REC=$(ls "$BB_W"/verifications/OBL-*.toml 2>/dev/null | wc -l)
if [[ $BB_STATUS -ne 0 ]] && [[ "$(grep -c 'verify.outside-bundle' <<<"$BB_RUN")" -eq 2 ]] && [[ "$BB_REC" -eq 0 ]]; then
    bb_ok "an answer beyond the bundle is refused" "verify.outside-bundle x2 (exit $BB_STATUS), nothing recorded"
else
    bb_fail "an answer beyond the bundle is refused" "exit $BB_STATUS, $BB_REC record(s): $(grep -E '^(ERROR|PASS verify.recorded)' <<<"$BB_RUN" | head -2)"
fi
command cp -r conformance/fixtures "$PLANT_ROOT/conformance-fixtures"
sed -i "s|^verifier_argv = .*|verifier_argv = [\"bash\", \"$PLANT_ROOT/conformance-fixtures/verifier/establishes-all.sh\"]|" "$PLANT_ROOT/openwarrant.toml"
BB_RUN=$("$WAR" --root "$PLANT_ROOT" verify BB-WAR-0001 --performer claude --run 2>&1); BB_STATUS=$?
BB_REC=$(ls "$BB_W"/verifications/OBL-*.toml 2>/dev/null | wc -l)
if [[ $BB_STATUS -eq 0 ]] && [[ "$(grep -c 'verify.recorded' <<<"$BB_RUN")" -eq 2 ]] && [[ "$BB_REC" -eq 2 ]]; then
    bb_ok "each obligation's bundle is ingested" "one verifier call per bundle, 2 records"
else
    bb_fail "each obligation's bundle is ingested" "exit $BB_STATUS, $BB_REC record(s): $(grep -E '^ERROR' <<<"$BB_RUN" | head -2)"
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT BB_W BB_A BB_OUT BB_RUN
