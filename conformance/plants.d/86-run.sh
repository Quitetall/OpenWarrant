# shellcheck shell=bash
# The ops / runs work kind (slice C4b): `war run` on a service stage, the
# receipt bound to the dispatch digest, the submission that can only ask to
# be verified or unblocked, and `war submit` refusing what §51.2 forbids.
# STAGE-001 of the run Warrant binds `ops.echo` (`true`) so a run takes no time.

RUN_ALIAS=$(grep -l 'title = "Run: the conformance battery as a service stage"' docs/warrants/*/manifest.toml | head -1 | cut -d/ -f3)
[[ -n "$RUN_ALIAS" ]] || { echo "the run Warrant is missing; the plants cannot run" >&2; exit 9; }
RUN_DIR=docs/warrants/$RUN_ALIAS
RUN_MS=$RUN_DIR/atoms/45-milestones.yaml
RUN_TMP=$(mktemp -d)
run_cleanup() { rm -rf "$RUN_DIR/dispatches" "$RUN_DIR/submissions" "$RUN_DIR/gate-runs"; restore; }

# Positive: the trivial gate runs, receipts, and submits `verify`.
plant_cmd "a service stage runs and submits verify" "run.passed" "ops.echo" 0 \
    "true" \
    run "$RUN_ALIAS" STAGE-001
SUB=$(ls "$RUN_DIR"/submissions/*.json 2>/dev/null | head -1)
if [[ -n "$SUB" ]] && grep -q '"requested_next_action": "verify"' "$SUB" && ls "$RUN_DIR"/gate-runs/*.receipt.json >/dev/null 2>&1 && grep -q 'dispatch:' "$RUN_DIR"/gate-runs/*.receipt.json; then
    printf 'ok    %-34s receipt subject is the dispatch digest; submission requests verify\n' "the run left its records"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s submission=%s\n' "the run left its records" "${SUB:-none}"
    FAILED=$((FAILED + 1))
fi
# Keep one real dispatch id for the submit plants, then clean.
RUN_DID=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['dispatch_id'])" "$SUB" 2>/dev/null)
cp "$SUB" "$RUN_TMP/sub.json" 2>/dev/null
run_cleanup

# A human stage cannot be run.
plant_cmd "a human stage is not run" "run.not-a-service" "human" 2 \
    "sed -i '0,/executor_kind: \"service\"/s//executor_kind: \"human\"/' $RUN_MS; assert_present 'executor_kind: \"human\"' $RUN_MS" \
    run "$RUN_ALIAS" STAGE-001
run_cleanup

# An executor that is not a registered gate is refused by name.
plant_cmd "an unknown gate is refused" "run.unknown-gate" "ops.nothing" 2 \
    "sed -i 's|gate://ops.echo@1.0.0|gate://ops.nothing@9.9.9|' $RUN_MS; assert_present 'ops.nothing' $RUN_MS" \
    run "$RUN_ALIAS" STAGE-001
run_cleanup

# Over wall time: the gate's argv is replaced by a sleep longer than the bound.
plant_cmd "a run over its wall time is a timeout" "run.timeout" "block" 2 \
    "sed -i 's|^argv: \\[\"true\"\\]|argv: [\"sleep\", \"3\"]|' docs/gates/ops.echo@1.0.0.yaml; sed -i 's|wall_time_seconds: 5|wall_time_seconds: 1|' $RUN_MS; assert_present 'sleep' docs/gates/ops.echo@1.0.0.yaml" \
    run "$RUN_ALIAS" STAGE-001
run_cleanup

# `war submit`: a submission requesting its own resolution is refused and NOT
# written; one naming a dispatch never compiled is refused; a sound one lands.
if [[ -n "$RUN_DID" ]]; then
    python3 - "$RUN_TMP/sub.json" "$RUN_TMP" <<'PY'
import json, sys
s = json.load(open(sys.argv[1])); out = sys.argv[2]
bad = dict(s); bad["requested_next_action"] = "resolve"; json.dump(bad, open(out + "/bad.json", "w"))
unk = dict(s); unk["requested_next_action"] = "continue"; unk["dispatch_id"] = "01a00000-0000-7000-8000-00000000c4b0"; json.dump(unk, open(out + "/unknown.json", "w"))
PY
    plant_cmd "a submission asking to be resolved is refused" "submission.self-completion" "resolve" 2 \
        "true" \
        submit "$RUN_ALIAS" "$RUN_TMP/bad.json"
    if [[ ! -f "$RUN_DIR/submissions/01a00000-0000-7000-8000-00000000c4b0.json" ]] && ! grep -q '"requested_next_action": "resolve"' "$RUN_DIR"/submissions/*.json 2>/dev/null; then
        printf 'ok    %-34s nothing written for the refused submission\n' "a refused submission is absent"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s a refused submission was written\n' "a refused submission is absent"
        FAILED=$((FAILED + 1))
    fi
    plant_cmd "a submission for an unknown dispatch is refused" "submission.unknown-dispatch" "dispatch.compiled" 2 \
        "true" \
        submit "$RUN_ALIAS" "$RUN_TMP/unknown.json"
    # A fresh compiled dispatch to answer, made AFTER the plant_cmd calls
    # above: each of them restores the tree, which wipes the journal event a
    # submission must name. Run directly for the same reason.
    "$WAR" run "$RUN_ALIAS" STAGE-001 >/dev/null 2>&1
    RUN_DID=$(python3 -c "import json,sys,glob; print(json.load(open(glob.glob(sys.argv[1]+'/submissions/*.json')[0]))['dispatch_id'])" "$RUN_DIR")
    python3 - "$RUN_TMP/sub.json" "$RUN_DID" "$RUN_TMP" <<'PY'
import json, sys
s = json.load(open(sys.argv[1])); s["dispatch_id"] = sys.argv[2]; s["requested_next_action"] = "continue"
json.dump(s, open(sys.argv[3] + "/good.json", "w"))
PY
    GOOD_OUT=$("$WAR" submit "$RUN_ALIAS" "$RUN_TMP/good.json" 2>&1)
    if [[ $? -eq 0 ]] && grep -q 'submission.recorded' <<< "$GOOD_OUT" && grep -q 'continue' <<< "$GOOD_OUT"; then
        printf 'ok    %-34s recorded, requesting continue\n' "a sound external submission is recorded"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s\n%s\n' "a sound external submission is recorded" "$(grep -E '^(ERROR|error)' <<< "$GOOD_OUT" | head -2)"
        FAILED=$((FAILED + 1))
    fi
    run_cleanup
fi
rm -rf "$RUN_TMP"
