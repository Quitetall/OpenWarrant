# shellcheck shell=bash
# Token accounting (slice C2, SAS §33.7): the packet carries its estimate and
# budget, an over-budget stage is refused naming the largest items, and a
# budget that is not a number is a malformed graph.

TOK_TMP=$(mktemp -d)
MS=docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml

# The packet carries `tokens` (estimate, budget, method) inside its digest.
"$WAR" dispatch OW-WAR-0047 STAGE-002 --emit "$TOK_TMP/d.json" >/dev/null 2>&1
restore
if python3 - "$TOK_TMP/d.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); t = d.get("tokens") or {}
sys.exit(0 if t.get("method") == "oh.war/token-estimate/bytes-div-4/v1" and t.get("estimated_tokens", 0) > 0 and t.get("budget_tokens", 0) >= t["estimated_tokens"] else 1)
PY
then
    printf 'ok    %-34s estimate, budget and method are in the packet\n' "the packet carries its token account"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "the packet carries its token account" "$(python3 -c "import json; print(json.load(open('$TOK_TMP/d.json')).get('tokens'))" 2>&1)"
    FAILED=$((FAILED + 1))
fi

# A stage whose budget is smaller than its context is refused, and the refusal
# names what to cut.
plant_cmd "an over-budget stage is refused" "dispatch.over-budget" "largest:" 2 \
    "sed -i 's|^    executor_ref: \"materialize_dataset_path\"$|    executor_ref: \"materialize_dataset_path\"\n    budget_tokens: 10|' $MS; assert_present 'budget_tokens: 10' $MS" \
    dispatch OW-WAR-0047 STAGE-002 --emit "$TOK_TMP/x.json"

# A budget that is not a non-negative integer is a malformed milestones atom.
plant "a non-numeric budget is refused" "milestones.invalid" "budget_tokens" 2 \
    "sed -i 's|^    executor_ref: \"materialize_dataset_path\"$|    executor_ref: \"materialize_dataset_path\"\n    budget_tokens: \"lots\"|' $MS; assert_present 'budget_tokens: \"lots\"' $MS" \
    OW-WAR-0047

# The compile is journalled with its size.
"$WAR" dispatch OW-WAR-0047 STAGE-002 --emit "$TOK_TMP/d2.json" >/dev/null 2>&1
if tail -1 docs/warrants/OW-WAR-0047/journal.jsonl | grep -q '"type":"dispatch.compiled"' && tail -1 docs/warrants/OW-WAR-0047/journal.jsonl | grep -q 'estimated_tokens'; then
    printf 'ok    %-34s the last journal event is dispatch.compiled with its estimate\n' "a compile is journalled"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "a compile is journalled" "$(tail -1 docs/warrants/OW-WAR-0047/journal.jsonl | cut -c1-160)"
    FAILED=$((FAILED + 1))
fi
restore

# ---------------------------------------------------------------------------
# OW-WAR-0129 (RQ-046, §47.2): the compiler is where the budget rule lives, and
# every path that starts an actor inherits it. These plants run on a scratch
# program, never on this corpus: TK-WAR-0002 there has three stages — an agent
# stage over budget (`budget_tokens: 10`), an agent stage in budget, and a
# service stage over budget — and a fixture performer that leaves a marker
# file named for the stage it was handed. A marker is what shows an actor
# started; its absence is what shows none did.
# ---------------------------------------------------------------------------

tok_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tok_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# OBL-001: the CLI keeps no comparison of its own; the refusal above came from
# the compiler.
if grep -q 'estimated_tokens > budget_tokens' crates/openwarrant-cli/src/dispatch.rs; then
    tok_fail "the CLI keeps no budget check" "crates/openwarrant-cli/src/dispatch.rs still compares estimated_tokens > budget_tokens"
elif grep -q 'OverBudget' crates/openwarrant-cli/src/dispatch.rs \
    && grep -q 'estimated_tokens > budget_tokens' crates/openwarrant-compiler/src/dispatch.rs; then
    tok_ok "the CLI keeps no budget check" "the comparison is the compiler's; the CLI maps OverBudget"
else
    tok_fail "the CLI keeps no budget check" "the compiler holds no comparison, or the CLI does not map its refusal"
fi

TOK_ROOT=$(scratch_corpus TK)
# Sourced outside the battery, `scratch_corpus` is undefined and every
# `git -C "$TOK_ROOT"` below would act on this repository. Refuse.
[[ -d "${TOK_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
TOK_W=TK-WAR-0002
TOK_DIR="$TOK_ROOT/docs/warrants/$TOK_W"
TOK_MS="$TOK_DIR/atoms/45-milestones.yaml"
TOK_MARKS="$TOK_TMP/marks"
mkdir -p "$TOK_MARKS"
"$WAR" --root "$TOK_ROOT" new "Budget plant" >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: war new in %s\n' "$TOK_ROOT" >&2; exit 9; }
[[ -d "$TOK_DIR" ]] || { printf 'PLANT SETUP FAILED: %s was not created\n' "$TOK_W" >&2; exit 9; }
# The scaffold's adopt Warrant has agent stages with no executor; under
# `perform --all` they would be refused for a reason that is not this plant's.
sed -i 's/executor_kind: "agent"/executor_kind: "human"/' "$TOK_ROOT/docs/warrants/TK-WAR-0001/atoms/45-milestones.yaml"
cat > "$TOK_MS" <<'YAML'
schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "budget plant"
    stage_refs: ["STAGE-001", "STAGE-002", "STAGE-003"]

stages:
  - id: "STAGE-001"
    title: "an agent stage over budget"
    executor_kind: "agent"
    responsibility_tier: "T2"
    executor_ref: "agent://fixture"
    budget_tokens: 10
  - id: "STAGE-002"
    title: "an agent stage in budget"
    executor_kind: "agent"
    responsibility_tier: "T2"
    executor_ref: "agent://fixture"
  - id: "STAGE-003"
    title: "a service stage over budget"
    executor_kind: "service"
    responsibility_tier: "T1"
    executor_ref: "gate://software.repo.war-check@1.0.0"
    budget_tokens: 10
YAML
# The fixture performer: writes its marker, then answers legally. `python3 -c`
# so the Dispatch on stdin reaches it.
cat > "$TOK_TMP/performer.sh" <<SH
#!/usr/bin/env bash
set -euo pipefail
exec python3 -c '
import json, sys
d = json.load(sys.stdin)
open("$TOK_MARKS/" + d["stage_id"], "w").write(d["dispatch_id"])
print(json.dumps({"schema": "oh.war/stage-submission/v1", "dispatch_id": d["dispatch_id"],
    "attempt_id": d["attempt_id"], "contract_digest": d["contract_digest"], "stage_id": d["stage_id"],
    "claims": [{"id": "C-001", "statement": "The fixture performer ran and asserts nothing else."}],
    "artifact_refs": [], "blockers": [], "requested_next_action": "verify"}))
'
SH
chmod +x "$TOK_TMP/performer.sh"
printf '\n[perform]\nperformer_argv = ["%s"]\nperformer_timeout_secs = 60\nmax_concurrent = 1\n' "$TOK_TMP/performer.sh" >> "$TOK_ROOT/openwarrant.toml"
"$WAR" --root "$TOK_ROOT" compile >/dev/null 2>&1
git -C "$TOK_ROOT" add -A >/dev/null 2>&1
git -C "$TOK_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "budget plant" >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: commit in %s\n' "$TOK_ROOT" >&2; exit 9; }
assert_present 'budget_tokens: 10' "$TOK_MS"
assert_present "$TOK_TMP/performer.sh" "$TOK_ROOT/openwarrant.toml"

tok_reset() { corpus_reset "$TOK_ROOT"; rm -f "$TOK_MARKS"/*; }
tok_compiled() { grep -c '"type":"dispatch.compiled"' "$TOK_DIR/journal.jsonl" 2>/dev/null || true; }
tok_packets() { find "$TOK_DIR/dispatches" -name '*.json' 2>/dev/null | wc -l; }
tok_receipts() { find "$TOK_DIR/gate-runs" -name '*.receipt.json' 2>/dev/null | wc -l; }
# Remove the budget from one stage, proving the edit landed.
tok_unbudget() {
    python3 - "$TOK_MS" "$1" <<'PY'
import sys
p, stage = sys.argv[1], sys.argv[2]
out, cur = [], None
for line in open(p).read().splitlines(keepends=True):
    if line.startswith("  - id: "):
        cur = line.split('"')[1]
    if cur == stage and line.strip() == "budget_tokens: 10":
        continue
    out.append(line)
open(p, "w").write("".join(out))
PY
}

# OBL-001: the unedited stage compiles, and its packet's account is exactly
# what the pre-compiler rule recorded — bytes of the selected items ÷ 4 rounded
# up, the repository default budget, the named method — recomputed here from
# the atoms on disk rather than read back from the tool.
tok_reset
TOK_OUT=$("$WAR" --root "$TOK_ROOT" dispatch "$TOK_W" STAGE-002 --prototype \
    --emit "$TOK_TMP/in.json" --emit-context "$TOK_TMP/in-ctx.json" 2>&1)
TOK_STATUS=$?
if [[ $TOK_STATUS -eq 0 ]] && python3 - "$TOK_TMP/in.json" "$TOK_TMP/in-ctx.json" "$TOK_DIR" <<'PY'
import json, os, sys
d, c, root = json.load(open(sys.argv[1])), json.load(open(sys.argv[2])), sys.argv[3]
total = sum(os.path.getsize(os.path.join(root, i["id"])) for i in c["included"])
want = {"estimated_tokens": -(-total // 4), "budget_tokens": 32000,
        "method": "oh.war/token-estimate/bytes-div-4/v1"}
sys.exit(0 if d.get("tokens") == want and all("#" not in i["id"] for i in c["included"]) else 1)
PY
then
    tok_ok "an in-budget stage is unchanged" "same estimate, budget and method as before the move"
else
    tok_fail "an in-budget stage is unchanged" "exit $TOK_STATUS; $(python3 -c "import json; print(json.load(open('$TOK_TMP/in.json')).get('tokens'))" 2>&1 | head -1)"
fi

# OBL-002: `war perform` over budget — refused by the compiler's rule, the
# performer never started, no packet on record, nothing journalled.
tok_reset
TOK_J0=$(tok_compiled)
TOK_OUT=$("$WAR" --root "$TOK_ROOT" perform "$TOK_W" STAGE-001 --prototype 2>&1)
TOK_STATUS=$?
if [[ $TOK_STATUS -ne 0 ]] && grep -q "dispatch.over-budget .*$TOK_W/STAGE-001" <<<"$TOK_OUT" \
    && grep -q 'largest:' <<<"$TOK_OUT" \
    && [[ ! -e "$TOK_MARKS/STAGE-001" ]] && [[ "$(tok_packets)" -eq 0 ]] \
    && [[ "$(tok_compiled)" == "$TOK_J0" ]] \
    && ! grep -q 'perform.answered' <<<"$TOK_OUT"; then
    tok_ok "perform over budget starts nothing" "rejected by dispatch.over-budget; no marker, packet or journal line"
else
    tok_fail "perform over budget starts nothing" "exit $TOK_STATUS; marker $([[ -e "$TOK_MARKS/STAGE-001" ]] && echo present || echo absent); packets $(tok_packets); compiled $TOK_J0 -> $(tok_compiled)"
fi

# The refusal is planted, not a no-op: the same stage without its budget is
# performed, and the performer leaves its marker.
tok_reset
tok_unbudget STAGE-001
if grep -A6 'id: "STAGE-001"' "$TOK_MS" | grep -q budget_tokens; then
    printf 'PLANT MUTATION WAS A NO-OP: STAGE-001 still carries a budget\n' >&2; exit 9
fi
TOK_OUT=$("$WAR" --root "$TOK_ROOT" perform "$TOK_W" STAGE-001 --prototype 2>&1)
TOK_STATUS=$?
if [[ $TOK_STATUS -eq 0 ]] && [[ -e "$TOK_MARKS/STAGE-001" ]] && grep -q 'perform.answered' <<<"$TOK_OUT" \
    && ! grep -q 'dispatch.over-budget' <<<"$TOK_OUT"; then
    tok_ok "perform in budget starts the actor" "the marker exists: the refusal above is the budget's"
else
    tok_fail "perform in budget starts the actor" "exit $TOK_STATUS; $(grep -m1 -E 'ERROR|FAIL' <<<"$TOK_OUT")"
fi

# OBL-002: `war perform --all` — the in-budget stage is performed, the
# over-budget one is named and not started. One refusal hides neither (R-002).
tok_reset
TOK_OUT=$("$WAR" --root "$TOK_ROOT" perform --all --prototype 2>&1)
TOK_STATUS=$?
if [[ $TOK_STATUS -ne 0 ]] && grep -q "dispatch.over-budget .*$TOK_W/STAGE-001" <<<"$TOK_OUT" \
    && grep -q "perform.answered .*$TOK_W/STAGE-002" <<<"$TOK_OUT" \
    && [[ -e "$TOK_MARKS/STAGE-002" ]] && [[ ! -e "$TOK_MARKS/STAGE-001" ]]; then
    tok_ok "perform --all skips only the over" "STAGE-002 performed; STAGE-001 named by dispatch.over-budget"
else
    tok_fail "perform --all skips only the over" "exit $TOK_STATUS; markers: $(ls "$TOK_MARKS" | tr '\n' ' ')"
fi

# OBL-002: `war run` over budget — refused before the gate runs, no receipt.
tok_reset
TOK_J0=$(tok_compiled)
TOK_OUT=$("$WAR" --root "$TOK_ROOT" run "$TOK_W" STAGE-003 --prototype 2>&1)
TOK_STATUS=$?
if [[ $TOK_STATUS -ne 0 ]] && grep -q "dispatch.over-budget .*$TOK_W/STAGE-003" <<<"$TOK_OUT" \
    && [[ "$(tok_receipts)" -eq 0 ]] && [[ ! -d "$TOK_DIR/gate-runs" ]] && [[ "$(tok_packets)" -eq 0 ]] \
    && [[ "$(tok_compiled)" == "$TOK_J0" ]]; then
    tok_ok "run over budget runs no gate" "rejected by dispatch.over-budget; no gate run, no receipt"
else
    tok_fail "run over budget runs no gate" "exit $TOK_STATUS; receipts $(tok_receipts); packets $(tok_packets)"
fi
# ... and without its budget the same stage runs the gate and mints a receipt,
# so "no receipt" above is the refusal's doing.
tok_reset
tok_unbudget STAGE-003
if grep -A6 'id: "STAGE-003"' "$TOK_MS" | grep -q budget_tokens; then
    printf 'PLANT MUTATION WAS A NO-OP: STAGE-003 still carries a budget\n' >&2; exit 9
fi
TOK_OUT=$("$WAR" --root "$TOK_ROOT" run "$TOK_W" STAGE-003 --prototype 2>&1)
if [[ "$(tok_receipts)" -ge 1 ]] && ! grep -q 'dispatch.over-budget' <<<"$TOK_OUT"; then
    tok_ok "run in budget mints a receipt" "the gate ran: the refusal above is the budget's"
else
    tok_fail "run in budget mints a receipt" "receipts $(tok_receipts); $(grep -m1 ERROR <<<"$TOK_OUT")"
fi

# OBL-003: a Dispatch from elsewhere is judged before it is bundled. The
# refusals are given a --context that does not exist: judged first, the
# packet is refused by its own rule before any context is read.
tok_reset
"$WAR" --root "$TOK_ROOT" dispatch "$TOK_W" STAGE-002 --prototype \
    --emit "$TOK_TMP/b.json" --emit-context "$TOK_TMP/b-ctx.json" >/dev/null 2>&1
python3 - "$TOK_TMP/b.json" "$TOK_TMP" <<'PY'
import json, sys
d, tmp = json.load(open(sys.argv[1])), sys.argv[2]
over = json.loads(json.dumps(d)); over["tokens"]["estimated_tokens"] = over["tokens"]["budget_tokens"] + 1
json.dump(over, open(f"{tmp}/b-over.json", "w"))
gone = json.loads(json.dumps(d)); del gone["tokens"]
json.dump(gone, open(f"{tmp}/b-gone.json", "w"))
PY
assert_present '"tokens"' "$TOK_TMP/b.json"
assert_gone '"tokens"' "$TOK_TMP/b-gone.json"
tok_bundle_refused() {
    local name="$1" rule="$2" packet="$3" out status
    rm -f "$TOK_TMP/refused.bundle"
    out=$("$WAR" --root "$TOK_ROOT" dispatch-bundle create "$TOK_W" --dispatch "$packet" \
        --context "$TOK_TMP/no-such-context.json" --emit "$TOK_TMP/refused.bundle" 2>&1)
    status=$?
    if [[ $status -ne 0 ]] && grep -q "$rule" <<<"$out" && [[ ! -e "$TOK_TMP/refused.bundle" ]] \
        && ! grep -q 'no-such-context' <<<"$out"; then
        tok_ok "$name" "rejected by $rule; nothing at --emit"
    else
        tok_fail "$name" "exit $status; $(head -1 <<<"$out")"
    fi
}
tok_bundle_refused "bundle over budget is refused" "bundle-over-budget" "$TOK_TMP/b-over.json"
tok_bundle_refused "bundle with no tokens is refused" "bundle-tokens-unrecorded" "$TOK_TMP/b-gone.json"
TOK_OUT=$("$WAR" --root "$TOK_ROOT" dispatch-bundle create "$TOK_W" --dispatch "$TOK_TMP/b.json" \
    --context "$TOK_TMP/b-ctx.json" --emit "$TOK_TMP/ok.bundle" 2>&1)
TOK_STATUS=$?
if [[ $TOK_STATUS -eq 0 ]] && [[ -s "$TOK_TMP/ok.bundle" ]] && grep -q 'dispatch-bundle.created' <<<"$TOK_OUT"; then
    tok_ok "the unedited Dispatch is bundled" "the refusals above are not blanket"
else
    tok_fail "the unedited Dispatch is bundled" "exit $TOK_STATUS; $(head -1 <<<"$TOK_OUT")"
fi

corpus_gone "$TOK_ROOT"
rm -rf "$TOK_TMP"
