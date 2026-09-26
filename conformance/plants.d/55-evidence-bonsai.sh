# shellcheck shell=bash
# t-dec1 — `war evidence record` can record the Bonsai evidence gate.
#
# Since OW-WAR-0133 evidence record binds the deliverable set beside the
# contract, and the Bonsai binding's one-subject rule refused every such
# run before the gate started: "Bonsai receipt binding requires exactly one
# contract subject and one evidence reference". The Bonsai gate verifies a
# supplied `war bonsai check` document; its receipt binds that document by
# bytes. So: the reference is passed (`--evidence-ref`), the receipt carries
# one contract subject with the deliverable and tree subjects beside it, and
# a run without the reference is refused by name with the remedy.
#
# Scratch program BZ. BZ-WAR-0001's first obligation is re-pointed at the
# Bonsai gate; the second still cites war-check. The passing document is a
# real one (OW-WAR-0111's) with its Warrant binding rewritten to BZ-WAR-0001's
# contract: the gate checks form and binding, not freshness (its declared
# blind spot), which is exactly what this plant exercises.

echo "== evidence record: the Bonsai gate (t-dec1) =="
BZ_ROOT=$(scratch_corpus BZ)
[[ -d "${BZ_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
PLANT_ROOT=$BZ_ROOT
BZ_W=BZ-WAR-0001
BZ_DIR="$BZ_ROOT/docs/warrants/$BZ_W"
BZ_RUNS="$BZ_DIR/gate-runs"
BZ_WAR="$REPO_ROOT/${WAR#./}"
BZ_GATE=software.repo.bonsai-evidence@1.0.0

bz_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
bz_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
bz_war()  { env -u SSH_AUTH_SOCK "$BZ_WAR" --root "$BZ_ROOT" "$@"; }
bz_commit() {
    git -C "$BZ_ROOT" add -A >/dev/null 2>&1
    git -C "$BZ_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1
}
bz_ref()  { printf 'file:%s#sha256:%s' "$1" "$(sha256sum "$BZ_ROOT/$1" | cut -d' ' -f1)"; }
bz_runs() { find "$BZ_RUNS" -type f 2>/dev/null | wc -l | tr -d ' '; }

# The gate definition as shipped, with its argv's war made absolute: the
# scratch program has no ./target of its own.
sed -e "s|\"./target/debug/war\"|\"$BZ_WAR\"|" \
    "$REPO_ROOT/docs/gates/$BZ_GATE.yaml" > "$BZ_ROOT/docs/gates/$BZ_GATE.yaml"
assert_present "$BZ_WAR" "$BZ_ROOT/docs/gates/$BZ_GATE.yaml"
sed -i "0,/gate:\/\/software.repo.war-check@1.0.0/s||gate://$BZ_GATE|" "$BZ_DIR/atoms/60-assurance.md"
assert_present "gate://$BZ_GATE" "$BZ_DIR/atoms/60-assurance.md"
bz_war compile >/dev/null 2>&1
bz_commit "cite the Bonsai gate" || { printf 'PLANT SETUP FAILED: commit in %s\n' "$BZ_ROOT" >&2; exit 9; }
BZ_C=$(bz_war --json status 2>/dev/null | python3 -c '
import json, sys
d = json.load(sys.stdin)
print(next(w["contract_digest"] for w in d["result"]["warrants"] if w["alias"] == sys.argv[1]))
' "$BZ_W")
[[ "$BZ_C" =~ ^[0-9a-f]{64}$ ]] || { printf 'PLANT SETUP FAILED: no contract digest for %s\n' "$BZ_W" >&2; exit 9; }
python3 - "$REPO_ROOT/docs/warrants/OW-WAR-0111/implementation/bonsai-3441573.json" "$BZ_C" "$BZ_W" \
    > "$BZ_ROOT/bonsai-evidence.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["warrant"]["contract_digest"] = sys.argv[2]
d["warrant"]["alias"] = sys.argv[3]
print(json.dumps(d, indent=2))
PY
cp "$BZ_ROOT/bonsai-evidence.json" "$BZ_ROOT/elsewhere.json"
python3 - "$BZ_ROOT/bonsai-evidence.json" > "$BZ_ROOT/other-contract.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["warrant"]["contract_digest"] = "0" * 64
print(json.dumps(d, indent=2))
PY
bz_commit "the Bonsai documents" || { printf 'PLANT SETUP FAILED: commit in %s\n' "$BZ_ROOT" >&2; exit 9; }

# ---- refused: no reference. Named rule, the remedy, nothing run or written.
BZ_OUT=$(bz_war evidence record "$BZ_W" --gate "$BZ_GATE" 2>&1); BZ_RC=$?
if [[ $BZ_RC -ne 0 ]] && grep -q 'evidence.bonsai-evidence-ref-required' <<<"$BZ_OUT" \
    && grep -q -- '--evidence-ref file:' <<<"$BZ_OUT" && [[ $(bz_runs) == 0 ]]; then
    bz_ok "bonsai: no reference, named" "evidence.bonsai-evidence-ref-required, remedy given, nothing written"
else
    bz_fail "bonsai: no reference, named" "exit $BZ_RC, $(bz_runs) file(s): $(grep -m1 -E 'ERROR|error' <<<"$BZ_OUT")"
fi

# ---- refused: a reference to a file the gate does not read.
BZ_OUT=$(bz_war evidence record "$BZ_W" --gate "$BZ_GATE" --evidence-ref "$(bz_ref elsewhere.json)" 2>&1); BZ_RC=$?
if [[ $BZ_RC -ne 0 ]] && grep -q 'gate-run.bonsai-ref-not-read' <<<"$BZ_OUT" && [[ $(bz_runs) == 0 ]]; then
    bz_ok "bonsai: reference not what it reads" "gate-run.bonsai-ref-not-read, nothing written"
else
    bz_fail "bonsai: reference not what it reads" "exit $BZ_RC, $(bz_runs) file(s): $(grep -m1 -E 'ERROR|error' <<<"$BZ_OUT")"
fi

# ---- refused: the digest is not the file's.
BZ_OUT=$(bz_war evidence record "$BZ_W" --gate "$BZ_GATE" \
    --evidence-ref "file:bonsai-evidence.json#sha256:$(printf x | sha256sum | cut -d' ' -f1)" 2>&1); BZ_RC=$?
if [[ $BZ_RC -ne 0 ]] && grep -q 'digest does not match file bytes' <<<"$BZ_OUT" && [[ $(bz_runs) == 0 ]]; then
    bz_ok "bonsai: a wrong digest" "refused, nothing written"
else
    bz_fail "bonsai: a wrong digest" "exit $BZ_RC, $(bz_runs) file(s): $(grep -m1 -E 'ERROR|error' <<<"$BZ_OUT")"
fi

# ---- refused: a passing document for another contract.
BZ_OUT=$(bz_war evidence record "$BZ_W" --gate "$BZ_GATE" --evidence-ref "$(bz_ref other-contract.json)" 2>&1); BZ_RC=$?
if [[ $BZ_RC -ne 0 ]] && grep -q 'for the bound contract digest' <<<"$BZ_OUT" && [[ $(bz_runs) == 0 ]]; then
    bz_ok "bonsai: another contract's report" "refused, nothing written"
else
    bz_fail "bonsai: another contract's report" "exit $BZ_RC, $(bz_runs) file(s): $(grep -m1 -E 'ERROR|error' <<<"$BZ_OUT")"
fi

# ---- refused: a Bonsai reference on another gate.
BZ_OUT=$(bz_war evidence record "$BZ_W" --gate software.repo.war-check@1.0.0 \
    --evidence-ref "$(bz_ref bonsai-evidence.json)" 2>&1); BZ_RC=$?
if [[ $BZ_RC -ne 0 ]] && grep -q 'is a Bonsai binding' <<<"$BZ_OUT" && [[ $(bz_runs) == 0 ]]; then
    bz_ok "bonsai: reference on another gate" "refused, nothing written"
else
    bz_fail "bonsai: reference on another gate" "exit $BZ_RC, $(bz_runs) file(s): $(grep -m1 -E 'ERROR|error' <<<"$BZ_OUT")"
fi

# ---- accepted: the document the gate reads, for this contract, committed.
# The receipt names one contract subject, the deliverable and tree subjects
# beside it, not dirty, and the document by bytes; `war check` admits it.
[[ -n "$(git -C "$BZ_ROOT" status --porcelain)" ]] && git -C "$BZ_ROOT" checkout -q -- . 2>/dev/null
BZ_REF=$(bz_ref bonsai-evidence.json)
BZ_OUT=$(bz_war evidence record "$BZ_W" --gate "$BZ_GATE" --evidence-ref "$BZ_REF" 2>&1); BZ_RC=$?
BZ_RECEIPT="$BZ_RUNS/software_repo_bonsai-evidence_1_0_0.receipt.json"
BZ_SHAPE=$(python3 - "$BZ_RECEIPT" "$BZ_C" "$BZ_REF" <<'PY' 2>&1
import json, sys
r = json.load(open(sys.argv[1]))
s = r["subject_digests"]
bad = []
if [x for x in s if x.startswith("contract:")] != ["contract:sha256:" + sys.argv[2]]:
    bad.append("contract subjects %r" % s)
if not any(x.startswith("deliverables:sha256:") for x in s):
    bad.append("no deliverables subject")
if not any(x.startswith("tree:") for x in s):
    bad.append("no tree subject")
if "worktree:dirty" in s:
    bad.append("dirty")
if r["raw_evidence_refs"] != [sys.argv[3]]:
    bad.append("refs %r" % r["raw_evidence_refs"])
if r["verdict"] != "pass":
    bad.append("verdict " + r["verdict"])
print("; ".join(bad) or "ok")
PY
)
bz_war compile >/dev/null 2>&1
bz_commit "record the Bonsai gate"
BZ_CHECK=$(bz_war check "$BZ_W" 2>&1)
if [[ $BZ_RC -eq 0 && "$BZ_SHAPE" == ok ]] \
    && grep -q "evidence.admissible .*$BZ_GATE" <<<"$BZ_CHECK"; then
    bz_ok "bonsai: recorded and admissible" "contract + deliverables + tree, the document by bytes"
else
    bz_fail "bonsai: recorded and admissible" "exit $BZ_RC, receipt: $BZ_SHAPE; $(grep -m1 -E 'evidence\.' <<<"$BZ_CHECK")"
fi

corpus_gone "$BZ_ROOT"
unset BZ_ROOT PLANT_ROOT
