# shellcheck shell=bash
# A Warrant-scoped gate (document.review@1.1.0): `<warrant>` in a gate's argv
# is the Warrant its run is recorded for, and nothing else.
#
# document.review@1.0.0's argv is `war document review` with no alias, which
# reviews every Warrant in the corpus. OW-WAR-0138's receipt, bound to its
# own contract and deliverables, read FAIL in prepare runs 3 and 4 although
# every one of 0138's own lines passed: the failures were 0065's, 0122's,
# 0126's, 0128's, 0135's and 0142's (t-4f521). 1.1.0 names the
# Warrant, and the runner substitutes it.
#
# On a scratch program: a gate `echo <warrant>`, recorded for its Warrant,
# runs `echo <alias>` — the stream and the receipt's arguments name the
# alias. The refusal: run for no Warrant, it is not askable (`not_run`), no
# process runs and no stream is written. On this repository, read-only: the
# same refusal for document.review@1.1.0 itself.

echo "== a Warrant-scoped gate names its Warrant =="
PLANT_ROOT=$(scratch_corpus DS)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
DS_W=$(grep -oE 'DS-WAR-[0-9]{4}' <<<"$(ls "$PLANT_ROOT/docs/warrants")" | head -1)
[[ -n "$DS_W" ]] || { printf 'PLANT SETUP FAILED: the scratch program has no Warrant\n' >&2; exit 9; }
DS_G="$PLANT_ROOT/docs/gates"
sed -e 's/^gate_id: .*/gate_id: "ops.warrant-echo"/' -e 's/^argv: .*/argv: ["echo", "<warrant>"]/' \
    "$DS_G/software.repo.war-check@1.0.0.yaml" >"$DS_G/ops.warrant-echo@1.0.0.yaml"
python3 - "$PLANT_ROOT/docs/warrants/$DS_W/atoms/60-assurance.md" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
i = s.index("### OBL-002")
s = s[:i] + s[i:].replace("gate://software.repo.war-check@1.0.0", "gate://ops.warrant-echo@1.0.0", 1)
open(p, "w").write(s)
PY
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a Warrant-scoped gate" >/dev/null 2>&1

# The refusal first, so the stream files below can only be the bound run's.
DS_OUT=$("$WAR" --root "$PLANT_ROOT" gate --run --gate ops.warrant-echo@1.0.0 2>&1); DS_RC=$?
DS_STREAMS=$(find "$PLANT_ROOT" -name 'ops_warrant-echo_1_0_0.stdout.txt' -not -path '*/.git/*' | head -1)
if [[ $DS_RC -ne 0 ]] && line_has -F 'gate-run.unaskable' -F 'not_run' <<<"$DS_OUT" \
    && line_has -F 'names <warrant>' -F 'war evidence record <alias>' <<<"$DS_OUT" \
    && ! line_has -E '^PASS gate-run' -E '.' <<<"$DS_OUT" && [[ -z "$DS_STREAMS" ]]; then
    printf 'ok    %-34s exit %s, unaskable (not_run), no stream written\n' "run for no Warrant: not run" "$DS_RC"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; streams [%s]; %s\n' "run for no Warrant: not run" "$DS_RC" "$DS_STREAMS" "$(grep -m1 'gate-run' <<<"$DS_OUT")"; FAILED=$((FAILED + 1))
fi

DS_OUT=$("$WAR" --root "$PLANT_ROOT" evidence record "$DS_W" --gate ops.warrant-echo@1.0.0 2>&1); DS_RC=$?
DS_RUNS="$PLANT_ROOT/docs/warrants/$DS_W/gate-runs"
DS_STDOUT=$(cat "$DS_RUNS/ops_warrant-echo_1_0_0.stdout.txt" 2>/dev/null)
DS_ARGS=$(python3 -c 'import json,sys; print(" ".join(json.load(open(sys.argv[1]))["arguments"]))' \
    "$DS_RUNS/ops_warrant-echo_1_0_0.receipt.json" 2>/dev/null)
if [[ $DS_RC -eq 0 && "$DS_STDOUT" == "$DS_W" && "$DS_ARGS" == "echo $DS_W" ]]; then
    printf 'ok    %-34s ran `%s`; stdout %s; receipt arguments `%s`\n' "recorded for a Warrant: bound" "$DS_ARGS" "$DS_STDOUT" "$DS_ARGS"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; stdout [%s]; arguments [%s]\n' "recorded for a Warrant: bound" "$DS_RC" "$DS_STDOUT" "$DS_ARGS"; FAILED=$((FAILED + 1))
fi
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT

# This repository, read-only: document.review@1.1.0 run for no Warrant is
# not asked, and 1.0.0 is left as it was (its argv still names no Warrant).
DS_OUT=$("$WAR" gate --run --gate document.review@1.1.0 2>&1); DS_RC=$?
if [[ $DS_RC -ne 0 ]] && line_has -F 'document.review@1.1.0: askability not_askable' -F '(not_run)' <<<"$DS_OUT"; then
    printf 'ok    %-34s exit %s, unaskable (not_run)\n' "document.review@1.1.0, no Warrant" "$DS_RC"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; %s\n' "document.review@1.1.0, no Warrant" "$DS_RC" "$(grep -m1 'document.review@1.1.0' <<<"$DS_OUT")"; FAILED=$((FAILED + 1))
fi
DS_A0=$(sed -n 's/^argv: //p' docs/gates/document.review@1.0.0.yaml)
DS_A1=$(sed -n 's/^argv: //p' docs/gates/document.review@1.1.0.yaml)
if [[ "$DS_A0" == '["./target/debug/war", "document", "review"]' && "$DS_A1" == '["./target/debug/war", "document", "review", "<warrant>"]' ]]; then
    printf 'ok    %-34s 1.0.0 %s; 1.1.0 %s\n' "the two argvs" "$DS_A0" "$DS_A1"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s 1.0.0 %s; 1.1.0 %s\n' "the two argvs" "$DS_A0" "$DS_A1"; FAILED=$((FAILED + 1))
fi
