# shellcheck shell=bash
# OW-WAR-0133 — evidence reuse after a source change, and the compiler's
# source, conflict and omission rules.
#
# Every case runs on a scratch program (ER), except OBL-003's second half,
# which reads this repository and mutates nothing. ER-WAR-0001 there cites one
# of two fixture gates, both `true`:
#
#   plant.reads@1.0.0  declares `inputs: ["src/**"]` and a fixture, so Q-001's
#                      (c) rule decides: the run holds while src/ and the
#                      fixture digest the same;
#   plant.tree@1.0.0   declares neither, so the (a) fallback decides: the run
#                      holds while nothing outside the evidence records and
#                      projections has changed since the tree it ran over.
#
# Each scenario records a run the way this repository does — record, compile,
# commit — so the committed receipt and the projections it moved are part of
# every control. A rule that refused every receipt would fail the first one.

ER_TMP=$(mktemp -d)
ER_ROOT=$(scratch_corpus ER)
# Sourced outside the battery, `scratch_corpus` is undefined and every
# `git -C "$ER_ROOT"` below would act on this repository. Refuse.
[[ -d "${ER_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
ER_W=ER-WAR-0001
ER_DIR="$ER_ROOT/docs/warrants/$ER_W"
ER_WAR="$REPO_ROOT/${WAR#./}"
ER_RUNS="$ER_DIR/gate-runs"

er_ok()      { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
er_fail()    { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# Law 15: a case this Warrant could not build is said to be unknown, and
# counted as neither.
er_unknown() { printf 'UNKNOWN %-32s %s\n' "$1" "$2"; }
er_war()     { "$ER_WAR" --root "$ER_ROOT" "$@"; }
er_commit()  {
    git -C "$ER_ROOT" add -A >/dev/null 2>&1
    git -C "$ER_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1
}
er_tree()    { git -C "$ER_ROOT" rev-parse 'HEAD^{tree}'; }

[[ -f "$ER_DIR/atoms/60-assurance.md" ]] \
    || { printf 'PLANT SETUP FAILED: %s has no assurance atom\n' "$ER_W" >&2; exit 9; }
for g in reads tree; do
    sed -e "s/^gate_id: .*/gate_id: \"plant.$g\"/" \
        "$REPO_ROOT/docs/gates/ops.echo@1.0.0.yaml" > "$ER_ROOT/docs/gates/plant.$g@1.0.0.yaml"
done
printf '\ninputs: ["src/**"]\nfixtures: ["fixtures/f.txt"]\n' >> "$ER_ROOT/docs/gates/plant.reads@1.0.0.yaml"
mkdir -p "$ER_ROOT/src" "$ER_ROOT/fixtures"
printf 'one\n' > "$ER_ROOT/src/lib.txt"
printf 'fixture\n' > "$ER_ROOT/fixtures/f.txt"
printf 'readme\n' > "$ER_ROOT/README.md"
cat > "$ER_DIR/deliverables.toml" <<'TOML'
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "The library the gates read"
kind = "file"
target_ref = "src/lib.txt"
required = true
content_addressed = false
provenance_required = false
obligation_refs = ["OBL-001"]
TOML
er_war compile >/dev/null 2>&1
er_commit "evidence-reuse setup" || { printf 'PLANT SETUP FAILED: commit in %s\n' "$ER_ROOT" >&2; exit 9; }
assert_present 'inputs: ["src/**"]' "$ER_ROOT/docs/gates/plant.reads@1.0.0.yaml"
assert_present 'gate://software.repo.war-check@1.0.0' "$ER_DIR/atoms/60-assurance.md"
ER_BASE=$(git -C "$ER_ROOT" rev-parse HEAD)

# er_scenario <reads|tree>: from the setup commit, cite one gate, record a run,
# compile, and commit it all. Leaves ER_T = the tree the run started from.
er_scenario() {
    git -C "$ER_ROOT" reset --hard -q "$ER_BASE" && git -C "$ER_ROOT" clean -fdq
    sed -i "s|gate://software.repo.war-check@1.0.0|gate://plant.$1@1.0.0|" "$ER_DIR/atoms/60-assurance.md"
    er_war compile >/dev/null 2>&1
    er_commit "cite plant.$1"
    ER_T=$(er_tree)
    ER_RECORD_OUT=$(er_war evidence record "$ER_W" 2>&1)
    er_war compile >/dev/null 2>&1
    er_commit "record plant.$1"
    ER_RECEIPT="$ER_RUNS/plant_$1_1_0_0.receipt.json"
}

# The receipt's seal recomputed here, outside the tool: RFC 8785 over
# {digest_domain, payload} with the digest field empty. Also used to RESEAL a
# receipt a plant rewrote, so that the rewrite is the only thing under test.
er_seal() {
    python3 - "$@" <<'PY'
import hashlib, json, sys
mode, path = sys.argv[1], sys.argv[2]
r = json.load(open(path))
def seal(r):
    u = dict(r, receipt_digest="")
    pre = json.dumps({"digest_domain": "oh.war/gate-run/v1", "payload": u},
                     sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return "sha256:" + hashlib.sha256(pre.encode()).hexdigest()
if mode == "verify":
    sys.exit(0 if seal(r) == r["receipt_digest"] else 1)
if mode == "contract-only":
    r["subject_digests"] = [s for s in r["subject_digests"] if s.startswith("contract:")]
    r["fixture_digests"] = []
    if len(sys.argv) > 3:
        r["receipt_digest"] = seal(r)
    json.dump(r, open(path, "w"), indent=2)
PY
}

# ---------------------------------------------------------------- OBL-001 ---
# A clean run names the contract, the tree it started from, the deliverable
# bytes, the declared inputs and the fixture; each digest is recomputed here
# from the files, not read back from the tool.
er_scenario reads
if python3 - "$ER_RECEIPT" "$ER_T" "$ER_ROOT" <<'PY'
import hashlib, json, sys
r, tree, root = json.load(open(sys.argv[1])), sys.argv[2], sys.argv[3]
h = lambda b: hashlib.sha256(b).hexdigest()
lib = h(open(f"{root}/src/lib.txt", "rb").read())
fx = h(open(f"{root}/fixtures/f.txt", "rb").read())
want = {
    f"tree:{tree}",
    "deliverables:sha256:" + h(f"D-001\0src/lib.txt\0{lib}\n".encode()),
    "inputs:sha256:" + h(f"src/lib.txt\0{lib}\n".encode()),
}
s = r["subject_digests"]
ok = (want <= set(s)
      and any(x.startswith("contract:sha256:") for x in s)
      and "worktree:dirty" not in s
      and r["fixture_digests"] == [f"fixtures/f.txt#sha256:{fx}"])
sys.exit(0 if ok else 1)
PY
then
    if er_seal verify "$ER_RECEIPT"; then
        er_ok "a clean receipt names its source" "contract, tree = HEAD^{tree}, deliverables, inputs, fixture; seal recomputes"
    else
        er_fail "a clean receipt names its source" "the subjects are right and the seal does not recompute"
    fi
else
    er_fail "a clean receipt names its source" "$(python3 -c "import json,sys; r=json.load(open(sys.argv[1])); print(r['subject_digests'], r['fixture_digests'])" "$ER_RECEIPT" 2>&1)"
fi

# A run over a dirty tree says so, and its seal still recomputes. Under the
# tree rule a dirty tree has no name: reuse-unknown, by that reason.
er_scenario tree
printf 'uncommitted\n' >> "$ER_ROOT/README.md"
er_war evidence record "$ER_W" >/dev/null 2>&1
ER_OUT=$(er_war check "$ER_W" 2>&1)
if grep -q '"worktree:dirty"' "$ER_RUNS/plant_tree_1_0_0.receipt.json" \
    && er_seal verify "$ER_RUNS/plant_tree_1_0_0.receipt.json" \
    && grep -q 'evidence.reuse-unknown .*dirty tree has no name' <<<"$ER_OUT" \
    && ! grep -q 'evidence.receipt-invalid' <<<"$ER_OUT"; then
    er_ok "a dirty receipt says so" "worktree:dirty named, seal recomputes; reuse-unknown (dirty tree has no name)"
else
    er_fail "a dirty receipt says so" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# Refusal: a receipt hand-edited to name another tree is not a receipt.
er_scenario tree
sed -i "s|\"tree:$ER_T\"|\"tree:0000000000000000000000000000000000000000\"|" "$ER_RUNS/plant_tree_1_0_0.receipt.json"
assert_present '"tree:0000000000000000000000000000000000000000"' "$ER_RUNS/plant_tree_1_0_0.receipt.json"
ER_OUT=$(er_war check "$ER_W" 2>&1)
if grep -q 'evidence.receipt-invalid .*does not recompute' <<<"$ER_OUT"; then
    er_ok "a receipt edited to a new tree" "rejected by evidence.receipt-invalid (does not recompute)"
else
    er_fail "a receipt edited to a new tree" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# ---------------------------------------------------------------- OBL-002 ---
ER_MET='resolution.requirement-met .*every required gate has admissible result'
ER_UNMET='resolution.requirement-unmet .*every required gate has admissible result'

# The recorded, committed run counts — through its own commit and the
# projections it moved.
er_scenario reads
ER_OUT=$(er_war check "$ER_W" 2>&1)
ER_RES=$(er_war resolve --dry-run "$ER_W" 2>&1)
if grep -q 'evidence.admissible .*plant.reads@1.0.0 .*inputs src/\*\*' <<<"$ER_OUT" && grep -q "$ER_MET" <<<"$ER_RES"; then
    er_ok "a committed run counts" "evidence.admissible under the inputs rule; requirement 5 met"
else
    er_fail "a committed run counts" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# One byte of the declared deliverable, committed: the same run is a record.
ER_INPUTS=$(grep -o '"inputs:sha256:[0-9a-f]*"' "$ER_RECEIPT" | tr -d '"')
printf 'two\n' > "$ER_ROOT/src/lib.txt"
er_commit "change the deliverable"
ER_OUT=$(er_war check "$ER_W" 2>&1)
ER_RES=$(er_war resolve --dry-run "$ER_W" 2>&1)
if [[ -n "$ER_INPUTS" ]] && grep -q "evidence.stale-binding .*names $ER_INPUTS" <<<"$ER_OUT" \
    && grep -q "$ER_UNMET" <<<"$ER_RES" && ! grep -q 'evidence.admissible' <<<"$ER_OUT"; then
    er_ok "a changed deliverable" "rejected by evidence.stale-binding naming $ER_INPUTS; requirement 5 unmet"
else
    er_fail "a changed deliverable" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# Control: a commit touching nothing the inputs rule reads.
er_scenario reads
printf 'more readme\n' >> "$ER_ROOT/README.md"
er_commit "unrelated"
ER_OUT=$(er_war check "$ER_W" 2>&1)
ER_RES=$(er_war resolve --dry-run "$ER_W" 2>&1)
if grep -q 'evidence.admissible .*plant.reads@1.0.0' <<<"$ER_OUT" && grep -q "$ER_MET" <<<"$ER_RES" \
    && ! grep -q 'evidence.stale-binding' <<<"$ER_OUT"; then
    er_ok "an unrelated commit leaves it" "evidence.admissible; requirement 5 still met"
else
    er_fail "an unrelated commit leaves it" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# A changed fixture is a changed question, whichever rule decides.
er_scenario reads
ER_FX=$(grep -o '"fixtures/f.txt#sha256:[0-9a-f]*"' "$ER_RECEIPT" | tr -d '"')
printf 'another fixture\n' > "$ER_ROOT/fixtures/f.txt"
er_commit "change the fixture"
ER_OUT=$(er_war check "$ER_W" 2>&1)
if [[ -n "$ER_FX" ]] && grep -q "evidence.stale-binding .*names $ER_FX" <<<"$ER_OUT"; then
    er_ok "a changed fixture" "rejected by evidence.stale-binding naming the fixture digest"
else
    er_fail "a changed fixture" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# The (a) fallback: a gate that declares no inputs holds through its own
# record commit, and not through any other.
er_scenario tree
ER_OUT=$(er_war check "$ER_W" 2>&1)
if grep -q "evidence.admissible .*plant.tree@1.0.0 .*tree:$ER_T" <<<"$ER_OUT"; then
    printf 'more readme\n' >> "$ER_ROOT/README.md"
    er_commit "unrelated"
    ER_OUT=$(er_war check "$ER_W" 2>&1)
    ER_RES=$(er_war resolve --dry-run "$ER_W" 2>&1)
    if grep -q "evidence.stale-binding .*names tree:$ER_T .*README.md" <<<"$ER_OUT" && grep -q "$ER_UNMET" <<<"$ER_RES"; then
        er_ok "the tree fallback" "admissible after its record commit; stale-binding naming tree:$ER_T after any other"
    else
        er_fail "the tree fallback" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
    fi
else
    er_fail "the tree fallback" "not admissible after its own record commit: $(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# ---------------------------------------------------------------- OBL-003 ---
# A receipt that names only the contract — every receipt minted before
# OW-WAR-0133 — resealed, so the missing source is the only thing wrong.
er_scenario tree
er_seal contract-only "$ER_RUNS/plant_tree_1_0_0.receipt.json" reseal
assert_gone "tree:$ER_T" "$ER_RUNS/plant_tree_1_0_0.receipt.json"
ER_OUT=$(er_war check "$ER_W" 2>&1)
ER_STATUS=$?
ER_RES=$(er_war resolve --dry-run "$ER_W" 2>&1)
if [[ $ER_STATUS -eq 0 ]] && grep -q 'WARN evidence.reuse-unknown .*names no tree' <<<"$ER_OUT" \
    && ! grep -qE 'evidence.(admissible|receipt-invalid|stale-binding)' <<<"$ER_OUT" \
    && grep -q "$ER_UNMET" <<<"$ER_RES"; then
    er_ok "a contract-only receipt" "evidence.reuse-unknown, a warning (check exits 0); requirement 5 unmet"
else
    er_fail "a contract-only receipt" "exit $ER_STATUS; $(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi
# ... and the same rewrite NOT resealed is refused by the seal, so the case
# above was judged on its subjects and not waved through.
er_scenario tree
er_seal contract-only "$ER_RUNS/plant_tree_1_0_0.receipt.json"
ER_OUT=$(er_war check "$ER_W" 2>&1)
if grep -q 'evidence.receipt-invalid .*does not recompute' <<<"$ER_OUT"; then
    er_ok "a contract-only rewrite, unsealed" "rejected by evidence.receipt-invalid (does not recompute)"
else
    er_fail "a contract-only rewrite, unsealed" "$(grep -m1 'evidence\.' <<<"$ER_OUT")"
fi

# This repository: no resolved Warrant is re-evaluated (OW-WAR-0010's
# contract-only receipt stays admissible as history), none gains an error or
# a reuse-unknown, and no resolution record is touched by checking.
ER_HASHES_BEFORE=$(sha256sum "$REPO_ROOT"/docs/warrants/*/resolution.toml 2>/dev/null)
ER_JSON=$("$ER_WAR" --root "$REPO_ROOT" --json check 2>/dev/null)
"$ER_WAR" --root "$REPO_ROOT" check OW-WAR-0010 >"$ER_TMP/r10" 2>&1
ER_HASHES_AFTER=$(sha256sum "$REPO_ROOT"/docs/warrants/*/resolution.toml 2>/dev/null)
cat > "$ER_TMP/resolved.py" <<'PY'
import glob, json, os, sys
root = sys.argv[1]
resolved = {os.path.basename(os.path.dirname(p))
            for p in glob.glob(f"{root}/docs/warrants/*/resolution.toml")}
d = json.load(sys.stdin)
bad = []
for x in d["diagnostics"]:
    alias = x["message"].split(":", 1)[0]
    if alias not in resolved:
        continue
    if x["severity"] == "error" or x["rule"] == "evidence.reuse-unknown":
        bad.append(f'{x["rule"]} {x["message"][:80]}')
if bad or not resolved:
    print(bad[:3] or "no resolved Warrant found")
    sys.exit(1)
PY
ER_WHY=$(python3 "$ER_TMP/resolved.py" "$REPO_ROOT" <<<"$ER_JSON" 2>&1)
ER_RC=$?
if [[ $ER_RC -eq 0 && -n "$ER_HASHES_BEFORE" && "$ER_HASHES_BEFORE" == "$ER_HASHES_AFTER" ]] \
    && grep -q 'evidence.admissible .*resolved: the source is not re-evaluated' "$ER_TMP/r10"; then
    er_ok "resolved history is untouched" "no error or reuse-unknown on a resolved Warrant; resolution.toml bytes unchanged"
else
    er_fail "resolved history is untouched" "${ER_WHY:-resolution.toml bytes moved, or OW-WAR-0010's receipt was re-evaluated}"
fi

# ---------------------------------------------------------- OBL-004, -005 ---
# ER-WAR-0002 carries one agent stage; the omission plant sets its budget
# below the required atoms.
git -C "$ER_ROOT" reset --hard -q "$ER_BASE" && git -C "$ER_ROOT" clean -fdq
er_war new "Context plant" >/dev/null 2>&1
ER_D2="$ER_ROOT/docs/warrants/ER-WAR-0002"
[[ -d "$ER_D2" ]] || { printf 'PLANT SETUP FAILED: ER-WAR-0002 was not created\n' >&2; exit 9; }
cat > "$ER_D2/atoms/45-milestones.yaml" <<'YAML'
schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "context plant"
    stage_refs: ["STAGE-001"]

stages:
  - id: "STAGE-001"
    title: "an agent stage"
    executor_kind: "agent"
    responsibility_tier: "T2"
    executor_ref: "agent://fixture"
    budget_tokens: 10
YAML
er_war compile >/dev/null 2>&1
er_commit "context plant"
ER_CTX_BASE=$(git -C "$ER_ROOT" rev-parse HEAD)

# OBL-005: a budget that cannot hold the required atoms refuses the Dispatch,
# naming them; nothing is emitted without them.
ER_OUT=$(er_war dispatch ER-WAR-0002 STAGE-001 --prototype --emit "$ER_TMP/d.json" --emit-context "$ER_TMP/c.json" 2>&1)
ER_STATUS=$?
if [[ $ER_STATUS -ne 0 ]] && grep -q 'dispatch.over-budget .*largest: atoms/' <<<"$ER_OUT" \
    && [[ ! -e "$ER_TMP/d.json" && ! -e "$ER_TMP/c.json" ]]; then
    er_ok "a budget too small for the atoms" "rejected by dispatch.over-budget naming required atoms; nothing emitted"
else
    er_fail "a budget too small for the atoms" "exit $ER_STATUS; $(grep -m1 -E 'ERROR|dispatch' <<<"$ER_OUT")"
fi
# ... control: without the budget the Dispatch compiles, every required atom
# included and none omitted.
sed -i '/budget_tokens: 10/d' "$ER_D2/atoms/45-milestones.yaml"
assert_gone 'budget_tokens: 10' "$ER_D2/atoms/45-milestones.yaml"
ER_OUT=$(er_war dispatch ER-WAR-0002 STAGE-001 --prototype --emit "$ER_TMP/d.json" --emit-context "$ER_TMP/c.json" 2>&1)
ER_STATUS=$?
if [[ $ER_STATUS -eq 0 ]] && python3 - "$ER_TMP/c.json" "$ER_D2/manifest.toml" <<'PY'
import json, re, sys
c = json.load(open(sys.argv[1]))
required = re.findall(r'path = "([^"]+)"\nrequired = true', open(sys.argv[2]).read())
ids = {i["id"] for i in c["included"]}
sys.exit(0 if required and set(required) <= ids and not any(o.get("required") for o in c["omitted"]) else 1)
PY
then
    er_ok "the same stage in budget" "every required atom included, none omitted"
else
    er_fail "the same stage in budget" "exit $ER_STATUS; $(grep -m1 -E 'ERROR|dispatch' <<<"$ER_OUT")"
fi
git -C "$ER_ROOT" reset --hard -q "$ER_CTX_BASE"

# OBL-004 and OBL-005's conflict half are not exercised here. The context
# manifest `war dispatch --emit-context` writes is built in
# crates/openwarrant-cli/src/dispatch.rs, and its `conflicts` field is typed
# in crates/openwarrant-core/src/context.rs; OW-WAR-0133 declares neither, so
# `conflicts: []` cannot be made to say `unchecked` inside it. The
# one-path-at-two-digests refusal is the compiler's (`source_conflicts`, unit
# tests in crates/openwarrant-compiler/src/dispatch.rs), and the stage
# selector never includes one path whole at two digests, so no stage a plant
# can write reaches it through the binary.
er_unknown "OBL-004 conflicts field" "not exercisable: the emitter is outside the declared set"
er_unknown "OBL-005 conflict via war dispatch" "not reachable through the stage selector; unit-tested in the compiler"

rm -rf "$ER_TMP"
