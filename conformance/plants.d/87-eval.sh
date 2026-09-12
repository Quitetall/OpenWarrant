# shellcheck shell=bash
# `war eval` (1.0 plan F1): the loop measured in scratch programs, scored on a
# ladder, deterministic for the fixture drafter, and compared per task.
# Nothing here touches the corpus: every scratch is a temp dir the harness
# removes, and results go to a temp file.

EVAL_TMP=$(mktemp -d)
trap 'rm -rf "$EVAL_TMP"' RETURN
EVAL_DRAFTER=evals/fixtures/fixture-agent.sh
EVAL_VERIFIER=conformance/fixtures/verifier/establishes-all.sh

eval_expect() {
    local name="$1" want_exit="$2" want="$3"
    shift 3
    local out status
    out=$("$WAR" "$@" 2>&1)
    status=$?
    if [[ $status -eq $want_exit ]] && { [[ -z "$want" ]] || grep -Fq -- "$want" <<< "$out"; }; then
        printf 'ok    %-34s exit %s %s\n' "$name" "$status" "$want"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s wanted exit %s %s; got exit %s:\n%s\n' "$name" "$want_exit" "$want" "$status" "$(grep -E '^(ERROR|WARN|error)' <<< "$out" | head -4)"
        FAILED=$((FAILED + 1))
    fi
}

# Positive: one task of each kind reaches would_satisfy with the fixture agent.
for task in code-01-changelog doc-01-tokenizer run-01-echo; do
    eval_expect "$task reaches would_satisfy" 0 "would_satisfy (expected would_satisfy)" \
        eval run --task "$task" --drafter "$EVAL_DRAFTER" --verifier "$EVAL_VERIFIER" --out "$EVAL_TMP/$task.json"
done

# A drafter that writes a file is REFUSED, not errored: the tool did its job.
eval_expect "a file-writing drafter scores refused" 0 "refused (expected would_satisfy)" \
    eval run --task code-01-changelog --drafter conformance/fixtures/drafter/writes-a-file.sh --verifier "$EVAL_VERIFIER" --out "$EVAL_TMP/refused.json"
if grep -q '"plan.drafter-wrote-files"' "$EVAL_TMP/refused.json" 2>/dev/null; then
    printf 'ok    %-34s the refusal is named in the record\n' "refused names its rule"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no plan.drafter-wrote-files in the record\n' "refused names its rule"
    FAILED=$((FAILED + 1))
fi

# A task whose budget the loop cannot fit is scored over_budget.
eval_expect "an unaffordable task is over_budget" 0 "over_budget (expected over_budget)" \
    eval run --tasks-dir conformance/fixtures/eval/tasks --drafter "$EVAL_DRAFTER" --verifier "$EVAL_VERIFIER" --out "$EVAL_TMP/budget.json"

# No drafter anywhere: a seam with nothing on the other side says so. The
# committed config names the real agent (OW-WAR-0042); it is cleared for this
# plant and restored after, so the battery never spends a model call.
plan_clear_drafter
eval_expect "no drafter is a named refusal" 1 "eval.no-drafter" \
    eval run --task code-01-changelog --out "$EVAL_TMP/none.json"
restore

# Determinism: two full runs of the fixture drafter are byte-identical.
"$WAR" eval run --drafter "$EVAL_DRAFTER" --verifier "$EVAL_VERIFIER" --out "$EVAL_TMP/run1.json" >/dev/null 2>&1
"$WAR" eval run --drafter "$EVAL_DRAFTER" --verifier "$EVAL_VERIFIER" --out "$EVAL_TMP/run2.json" >/dev/null 2>&1
# A run-kind task's bundle carries its receipt, and a receipt carries
# wall-clock durations, so its token estimate is not a function of the task;
# the result says so (`tokens.stable = false`) and the comparison drops those
# numbers and nothing else.
eval_stable() {
    python3 -c '
import json, sys
r = json.load(open(sys.argv[1]))
for t in r["tasks"]:
    if not t["tokens"].get("stable", True):
        t["tokens"] = {k: v for k, v in t["tokens"].items() if k in ("method", "stable")}
json.dump(r, open(sys.argv[2], "w"), sort_keys=True)
' "$1" "$2"
}
eval_stable "$EVAL_TMP/run1.json" "$EVAL_TMP/run1.stable.json"
eval_stable "$EVAL_TMP/run2.json" "$EVAL_TMP/run2.stable.json"
if cmp -s "$EVAL_TMP/run1.stable.json" "$EVAL_TMP/run2.stable.json" && [[ -s "$EVAL_TMP/run1.json" ]]; then
    printf 'ok    %-34s two runs, one result\n' "the fixture result is deterministic"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s the two results differ:\n%s\n' "the fixture result is deterministic" \
        "$(diff <(python3 -m json.tool "$EVAL_TMP/run1.stable.json" 2>/dev/null) <(python3 -m json.tool "$EVAL_TMP/run2.stable.json" 2>/dev/null) | head -6)"
    FAILED=$((FAILED + 1))
fi

# The committed baseline is this run (timings live beside it, not in it).
eval_expect "the baseline matches the run" 0 "eval.same" eval verify "$EVAL_TMP/run1.json"
# A regressed task is an ERROR by name.
python3 - "$EVAL_TMP/run1.json" "$EVAL_TMP/regressed.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
t = r["tasks"][0]; t["score"] = "partial"; t["as_expected"] = False
json.dump(r, open(sys.argv[2], "w"))
PY
eval_expect "a regressed task is named" 2 "eval.regressed" eval verify "$EVAL_TMP/regressed.json"

rm -rf "$EVAL_TMP"
