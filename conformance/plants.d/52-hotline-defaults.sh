# shellcheck shell=bash
# OW-WAR-0132: a blocking question stops its stage and only its stage; no
# responder is UNKNOWN; spend is unknown, never zero, and a cap nothing can
# enforce refuses; repairs and recoveries are counted apart and bounded.
#
# Every case runs on a scratch program, never this corpus. HD-WAR-0002 there
# has two open agent stages (the obligations' scope names OW-WAR-0068's; a
# scratch Warrant of the same shape is used because `war perform --all` over
# this corpus would perform every open agent stage in it). The performers are
# fixtures only: conformance/fixtures/performer/echo-submission.sh (answers
# legally), says-nothing.sh (fails), and a wrapper around echo-submission.sh
# that first leaves a marker named for its stage, so "spawned nothing" is an
# observation and not an inference.
#
# Each case checks WHICH rule fired, and each refusal is paired with the case
# the same control admits.

HD_TMP=$(mktemp -d)
HD_ROOT=$(scratch_corpus HD)
# Sourced outside the battery, `scratch_corpus` is undefined and every
# `git -C "$HD_ROOT"` below would act on this repository. Refuse.
[[ -d "${HD_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
HD_W=HD-WAR-0002
HD_DIR="$HD_ROOT/docs/warrants/$HD_W"
HD_Q="$HD_DIR/questions"
HD_ROLES="$HD_ROOT/docs/authority/roles.toml"
HD_CFG="$HD_ROOT/openwarrant.toml"
HD_MARKS="$HD_TMP/marks"
HD_FX="$REPO_ROOT/conformance/fixtures/performer"
HD_WAR="$REPO_ROOT/${WAR#./}"
mkdir -p "$HD_MARKS"

hd_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
hd_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
hd_war()  { "$HD_WAR" --root "$HD_ROOT" "$@"; }

"$HD_WAR" --root "$HD_ROOT" new "Hotline defaults plant" > /dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: war new in %s\n' "$HD_ROOT" >&2; exit 9; }
[[ -d "$HD_DIR" ]] || { printf 'PLANT SETUP FAILED: %s was not created\n' "$HD_W" >&2; exit 9; }
# The scaffold's adopt Warrant has agent stages of its own; none is this
# plant's, and `--all` must not perform them.
sed -i 's/executor_kind: "agent"/executor_kind: "human"/' "$HD_ROOT/docs/warrants/HD-WAR-0001/atoms/45-milestones.yaml"
cat > "$HD_DIR/atoms/45-milestones.yaml" << 'YAML'
schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "hotline defaults plant"
    stage_refs: ["STAGE-001", "STAGE-002"]

stages:
  - id: "STAGE-001"
    title: "the stage a question is asked against"
    executor_kind: "agent"
    responsibility_tier: "T2"
    executor_ref: "agent://fixture"
  - id: "STAGE-002"
    title: "an independent stage"
    executor_kind: "agent"
    responsibility_tier: "T2"
    executor_ref: "agent://fixture"
YAML
# A human who can answer, and an agent who cannot.
mkdir -p "$HD_ROOT/docs/authority"
cat > "$HD_ROLES" << 'TOML'
[[assignment]]
actor = "Plant Human"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "Plant Human"
effective_time = "2026-01-01T00:00:00Z"

[[assignment]]
actor = "claude"
actor_kind = "agent"
roles = ["performer"]
assigned_by = "Plant Human"
effective_time = "2026-01-01T00:00:00Z"
TOML
# The marker performer: records which stage it was handed, then answers as
# echo-submission.sh does. The Dispatch is read once and passed on.
cat > "$HD_TMP/marker-performer.sh" << SH
#!/usr/bin/env bash
set -euo pipefail
d=\$(cat)
stage=\$(python3 -c 'import json, sys; print(json.loads(sys.argv[1])["stage_id"])' "\$d")
touch "$HD_MARKS/\$stage"
printf '%s' "\$d" | exec "$HD_FX/echo-submission.sh"
SH
chmod +x "$HD_TMP/marker-performer.sh"
printf '\n[perform]\nperformer_argv = ["%s"]\nperformer_timeout_secs = 60\nmax_concurrent = 1\nallow_unmetered = true\n' \
    "$HD_TMP/marker-performer.sh" >> "$HD_CFG"
"$HD_WAR" --root "$HD_ROOT" compile > /dev/null 2>&1
git -C "$HD_ROOT" add -A > /dev/null 2>&1
git -C "$HD_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "hotline defaults plant" > /dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: commit in %s\n' "$HD_ROOT" >&2; exit 9; }
assert_present "$HD_TMP/marker-performer.sh" "$HD_CFG"
assert_present 'allow_unmetered = true' "$HD_CFG"
assert_present 'actor_kind = "human"' "$HD_ROLES"
assert_present 'id: "STAGE-002"' "$HD_DIR/atoms/45-milestones.yaml"

# --- helpers -----------------------------------------------------------------

hd_reset() { corpus_reset "$HD_ROOT"; rm -f "$HD_MARKS"/*; }
hd_marked() { [[ -e "$HD_MARKS/$1" ]]; }
hd_packets() { find "$HD_DIR/dispatches" -name '*.json' ! -name 'answer-*' 2> /dev/null | wc -l; }
# Ask a question against STAGE-001; echoes its id.
hd_ask() {
    hd_war ask "$HD_W" STAGE-001 "Planted: $1" "${@:2}" 2>&1 | grep -o 'Q-[0-9]\{3\}' | head -1
}
# The frontier row for a stage, as "state|waiting_on joined by commas".
hd_row() {
    python3 - "$1" "$2" << 'PY'
import json, sys
d = json.load(open(sys.argv[1]))
for r in d["result"]["rows"]:
    if r["warrant"] == "HD-WAR-0002" and r["stage"] == sys.argv[2]:
        print(r["state"] + "|" + ",".join(r.get("waiting_on", [])))
        break
PY
}
# The diagnostics with this rule in a report envelope, as "SEVERITY file".
hd_diag() {
    python3 - "$1" "$2" << 'PY'
import json, sys
d = json.load(open(sys.argv[1]))
for x in d.get("diagnostics", []):
    if x.get("rule") == sys.argv[2]:
        print(x.get("severity", ""), x.get("file") or "")
PY
}
# This stage's perform.ended outcomes, in journal order, comma-separated.
hd_endings() {
    python3 - "$HD_DIR/journal.jsonl" "$1" << 'PY'
import json, sys
out = []
for line in open(sys.argv[1]):
    line = line.strip()
    if not line:
        continue
    e = json.loads(line)
    if e.get("type") != "perform.ended":
        continue
    p = json.loads(e["payload"])
    if p.get("stage") == sys.argv[2]:
        out.append(p["outcome"])
print(",".join(out))
PY
}
# Rewrite one [perform] key (or drop it with an empty value), proving it landed.
hd_set() {
    python3 - "$HD_CFG" "$1" "$2" << 'PY'
import re, sys
p, key, value = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
s = re.sub(r'^' + re.escape(key) + r' = .*\n', '', s, flags=re.M)
if value:
    s = s.replace("[perform]\n", "[perform]\n" + key + " = " + value + "\n", 1)
open(p, "w").write(s)
PY
}
hd_perform() { hd_war perform "$HD_W" "$1" --prototype 2>&1; }

# --- OBL-001: an open blocking question stops its stage, and only its stage --

hd_reset
HD_QID=$(hd_ask "which way?" --blocking)
hd_war --json frontier > "$HD_TMP/f.json" 2> /dev/null
HD_R1=$(hd_row "$HD_TMP/f.json" STAGE-001)
HD_R2=$(hd_row "$HD_TMP/f.json" STAGE-002)
if [[ -n "$HD_QID" && "$HD_R1" == "blocked|$HD_QID" && "$HD_R2" == "open|" ]]; then
    hd_ok "a blocking question blocks" "STAGE-001 blocked waiting_on $HD_QID; STAGE-002 open"
else
    hd_fail "a blocking question blocks" "question '$HD_QID'; STAGE-001 '$HD_R1'; STAGE-002 '$HD_R2'"
fi
HD_OUT=$(hd_perform STAGE-001)
HD_STATUS=$?
if [[ $HD_STATUS -ne 0 ]] && grep -q "perform.question-open" <<< "$HD_OUT" \
    && grep -q "$HD_W/STAGE-001 waits on blocking question $HD_QID" <<< "$HD_OUT" \
    && ! hd_marked STAGE-001 && [[ "$(hd_packets)" -eq 0 ]]; then
    hd_ok "perform on a question-blocked stage" "rejected by perform.question-open ($HD_QID); no marker, no Dispatch"
else
    hd_fail "perform on a question-blocked stage" "exit $HD_STATUS; marker $(hd_marked STAGE-001 && echo present || echo absent); packets $(hd_packets); $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi
# Only its stage: --all performs the other one and leaves this one alone.
HD_OUT=$(hd_war perform --all --prototype 2>&1)
if hd_marked STAGE-002 && ! hd_marked STAGE-001 && grep -q "perform.answered" <<< "$HD_OUT" \
    && [[ "$(hd_endings STAGE-002)" == "answered" && -z "$(hd_endings STAGE-001)" ]]; then
    hd_ok "--all performs the other stage" "STAGE-002 answered; STAGE-001 not started"
else
    hd_fail "--all performs the other stage" "markers: $(ls "$HD_MARKS" | tr '\n' ' '); endings 1='$(hd_endings STAGE-001)' 2='$(hd_endings STAGE-002)'"
fi

# The rule is not blanket: a non-blocking question on the same stage blocks nothing.
hd_reset
HD_QID=$(hd_ask "just confirming")
hd_war --json frontier > "$HD_TMP/f.json" 2> /dev/null
HD_R1=$(hd_row "$HD_TMP/f.json" STAGE-001)
HD_OUT=$(hd_perform STAGE-001)
if [[ -n "$HD_QID" && "$HD_R1" == "open|" ]] && hd_marked STAGE-001 \
    && ! grep -q "perform.question-open" <<< "$HD_OUT" && grep -q "perform.answered" <<< "$HD_OUT"; then
    hd_ok "a non-blocking question" "STAGE-001 open and performed despite $HD_QID"
else
    hd_fail "a non-blocking question" "question '$HD_QID'; row '$HD_R1'; $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi

# Answered by a human, the stage is open again and war perform runs.
hd_reset
HD_QID=$(hd_ask "which way?" --blocking)
HD_ANS=$(hd_war answer "$HD_W" "$HD_QID" "This way." --as "Plant Human" 2>&1)
hd_war --json frontier > "$HD_TMP/f.json" 2> /dev/null
HD_R1=$(hd_row "$HD_TMP/f.json" STAGE-001)
HD_OUT=$(hd_perform STAGE-001)
if grep -q "question.answered" <<< "$HD_ANS" && [[ "$HD_R1" == "open|" ]] && hd_marked STAGE-001 \
    && grep -q "perform.answered" <<< "$HD_OUT"; then
    hd_ok "an answered question unblocks" "STAGE-001 open after $HD_QID answered; performed"
else
    hd_fail "an answered question unblocks" "row '$HD_R1'; $(head -1 <<< "$HD_ANS"); $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi

# --- OBL-002: no responder is UNKNOWN, never silent and never answered -------

hd_agents_only() {
    python3 - "$HD_ROLES" << 'PY'
import sys
p = sys.argv[1]
blocks = open(p).read().split("[[assignment]]")
keep = [b for b in blocks[1:] if 'actor_kind = "agent"' in b]
open(p, "w").write("".join("[[assignment]]" + b for b in keep))
PY
}
hd_reset
HD_QID=$(hd_ask "who answers this?" --blocking)
hd_agents_only
assert_gone 'actor_kind = "human"' "$HD_ROLES"
hd_war --json frontier > "$HD_TMP/f.json" 2> /dev/null
HD_R1=$(hd_row "$HD_TMP/f.json" STAGE-001)
HD_D=$(hd_diag "$HD_TMP/f.json" question.no-responder)
if [[ "$HD_D" == "unknown docs/authority/roles.toml" && "$HD_R1" == "blocked|$HD_QID" ]]; then
    hd_ok "frontier with no responder" "UNKNOWN question.no-responder naming roles.toml; STAGE-001 still blocked"
else
    hd_fail "frontier with no responder" "finding '$HD_D'; row '$HD_R1'"
fi
# `war next` must say the same. It does not: next.rs carries no diagnostics
# and is outside OW-WAR-0132's declared set. This plant stays, and fails,
# until the finding reaches `war next`.
hd_war --json next > "$HD_TMP/n.json" 2> /dev/null
HD_D=$(hd_diag "$HD_TMP/n.json" question.no-responder)
if [[ "$HD_D" == "unknown docs/authority/roles.toml" ]]; then
    hd_ok "next with no responder" "UNKNOWN question.no-responder naming roles.toml"
else
    hd_fail "next with no responder" "war next reports no question.no-responder (finding '$HD_D')"
fi
# The agent still cannot answer, and the question is untouched.
HD_SUM0=$(sha256sum "$HD_Q/$HD_QID.toml" | cut -d' ' -f1)
HD_OUT=$(hd_war answer "$HD_W" "$HD_QID" "I, an agent, answer myself" --as claude 2>&1)
HD_STATUS=$?
HD_SUM1=$(sha256sum "$HD_Q/$HD_QID.toml" | cut -d' ' -f1)
if [[ $HD_STATUS -ne 0 ]] && grep -q "question.agent" <<< "$HD_OUT" && [[ "$HD_SUM0" == "$HD_SUM1" ]]; then
    hd_ok "an agent answering, no responder" "rejected by question.agent; question byte-identical"
else
    hd_fail "an agent answering, no responder" "exit $HD_STATUS; digest $HD_SUM0 -> $HD_SUM1; $(head -1 <<< "$HD_OUT")"
fi
# With the human restored, the finding is gone and the stage still waits.
git -C "$HD_ROOT" checkout -q -- docs/authority/roles.toml
assert_present 'actor_kind = "human"' "$HD_ROLES"
hd_war --json frontier > "$HD_TMP/f.json" 2> /dev/null
HD_R1=$(hd_row "$HD_TMP/f.json" STAGE-001)
HD_D=$(hd_diag "$HD_TMP/f.json" question.no-responder)
if [[ -z "$HD_D" && "$HD_R1" == "blocked|$HD_QID" ]]; then
    hd_ok "frontier with a responder" "no question.no-responder; STAGE-001 blocked on $HD_QID"
else
    hd_fail "frontier with a responder" "finding '$HD_D'; row '$HD_R1'"
fi

# --- OBL-003: spend is unknown, never zero, and an unenforceable cap refuses --

hd_reset
hd_set hard_spend_cap '"1.00 USD"'
assert_present 'hard_spend_cap = "1.00 USD"' "$HD_CFG"
HD_J0=$(grep -c '"type":"dispatch.compiled"' "$HD_DIR/journal.jsonl" 2> /dev/null || true)
HD_OUT=$(hd_perform STAGE-001)
HD_STATUS=$?
HD_J1=$(grep -c '"type":"dispatch.compiled"' "$HD_DIR/journal.jsonl" 2> /dev/null || true)
if [[ $HD_STATUS -ne 0 ]] && grep -q "perform.spend-unenforceable" <<< "$HD_OUT" \
    && grep -q 'hard_spend_cap = "1.00 USD"' <<< "$HD_OUT" \
    && ! hd_marked STAGE-001 && [[ "$(hd_packets)" -eq 0 && "$HD_J0" == "$HD_J1" ]]; then
    hd_ok "a hard spend cap" "rejected by perform.spend-unenforceable; nothing compiled or spawned"
else
    hd_fail "a hard spend cap" "exit $HD_STATUS; marker $(hd_marked STAGE-001 && echo present || echo absent); packets $(hd_packets); compiled $HD_J0 -> $HD_J1"
fi

hd_reset
hd_set allow_unmetered ''
assert_gone 'allow_unmetered' "$HD_CFG"
HD_OUT=$(hd_perform STAGE-001)
HD_STATUS=$?
if [[ $HD_STATUS -ne 0 ]] && grep -q "perform.unmetered-not-allowed" <<< "$HD_OUT" \
    && grep -q 'allow_unmetered = true' <<< "$HD_OUT" \
    && ! hd_marked STAGE-001 && [[ "$(hd_packets)" -eq 0 ]]; then
    hd_ok "allow_unmetered absent" "rejected by perform.unmetered-not-allowed, naming the key"
else
    hd_fail "allow_unmetered absent" "exit $HD_STATUS; marker $(hd_marked STAGE-001 && echo present || echo absent); $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi
# The pair: with it true (the baseline), the stage runs, and its ending says
# spend is unknown.
hd_reset
HD_OUT=$(hd_perform STAGE-001)
if hd_marked STAGE-001 && grep -q "perform.answered" <<< "$HD_OUT" \
    && [[ "$(hd_endings STAGE-001)" == "answered" ]]; then
    hd_ok "allow_unmetered = true" "STAGE-001 performed; perform.ended answered"
else
    hd_fail "allow_unmetered = true" "endings '$(hd_endings STAGE-001)'; $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi

# --- OBL-004: repairs and recoveries are counted apart and bounded -----------
# The journal of each run is kept in $HD_TMP for the spend check after them.

hd_keep() { cp "$HD_DIR/journal.jsonl" "$HD_TMP/journal-$1.jsonl"; }
hd_run3() {
    # Three performances of STAGE-001 with the performers named; the third's
    # output and exit are left in HD_OUT and HD_STATUS.
    local i fx
    i=0
    for fx in "$@"; do
        i=$((i + 1))
        hd_set performer_argv "[\"$fx\"]"
        rm -f "$HD_MARKS"/*
        HD_OUT=$(hd_perform STAGE-001)
        HD_STATUS=$?
        [[ $i -lt 3 ]] && HD_PREV="$HD_PREV|$(grep -m1 -oE 'perform\.(answered|failed|[a-z-]+limit)' <<< "$HD_OUT")"
    done
}

hd_reset
HD_PREV=""
hd_set max_recoveries 1
assert_present 'max_recoveries = 1' "$HD_CFG"
hd_run3 "$HD_FX/says-nothing.sh" "$HD_FX/says-nothing.sh" "$HD_FX/says-nothing.sh"
HD_E=$(hd_endings STAGE-001)
hd_keep recoveries
if [[ $HD_STATUS -ne 0 ]] && grep -q "perform.recovery-limit" <<< "$HD_OUT" \
    && ! grep -q "perform.failed" <<< "$HD_OUT" && [[ "$HD_E" == "failed,failed" ]]; then
    hd_ok "max_recoveries = 1" "failed, failed, then rejected by perform.recovery-limit"
else
    hd_fail "max_recoveries = 1" "exit $HD_STATUS; endings '$HD_E'; earlier '$HD_PREV'; $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi

hd_reset
HD_PREV=""
hd_set max_repairs 1
assert_present 'max_repairs = 1' "$HD_CFG"
hd_run3 "$HD_FX/echo-submission.sh" "$HD_FX/echo-submission.sh" "$HD_FX/echo-submission.sh"
HD_E=$(hd_endings STAGE-001)
hd_keep repairs
if [[ $HD_STATUS -ne 0 ]] && grep -q "perform.repair-limit" <<< "$HD_OUT" \
    && ! grep -q "perform.answered" <<< "$HD_OUT" && [[ "$HD_E" == "answered,answered" ]]; then
    hd_ok "max_repairs = 1" "answered, answered, then rejected by perform.repair-limit"
else
    hd_fail "max_repairs = 1" "exit $HD_STATUS; endings '$HD_E'; earlier '$HD_PREV'; $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi

# A failed run does not consume a repair: answered, failed, answered is permitted.
hd_reset
HD_PREV=""
hd_set max_repairs 1
assert_present 'max_repairs = 1' "$HD_CFG"
hd_run3 "$HD_FX/echo-submission.sh" "$HD_FX/says-nothing.sh" "$HD_FX/echo-submission.sh"
HD_E=$(hd_endings STAGE-001)
hd_keep mixed
if [[ $HD_STATUS -eq 0 ]] && grep -q "perform.answered" <<< "$HD_OUT" \
    && ! grep -qE "perform\.(repair|recovery)-limit" <<< "$HD_OUT" && [[ "$HD_E" == "answered,failed,answered" ]]; then
    hd_ok "a failed run is not a repair" "answered, failed, answered; no limit fired"
else
    hd_fail "a failed run is not a repair" "exit $HD_STATUS; endings '$HD_E'; earlier '$HD_PREV'; $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$HD_OUT")"
fi

# --- OBL-003 again: every perform.ended above says spend is unknown ----------
# Over the kept journals: seven endings, answered and failed both. The grep for
# a zero spend is over the raw lines, so it cannot be fooled by the parse.
HD_SPEND=$(python3 - "$HD_TMP"/journal-*.jsonl << 'PY'
import json, sys
n, bad = 0, 0
for path in sys.argv[1:]:
    for line in open(path):
        line = line.strip()
        if not line:
            continue
        e = json.loads(line)
        if e.get("type") != "perform.ended":
            continue
        n += 1
        if json.loads(e["payload"]).get("spend") != "unknown":
            bad += 1
print(n, bad)
PY
)
HD_ZERO='\\"spend\\":(0|\\"0\\")'
if [[ "$HD_SPEND" == "7 0" ]] && ! grep -qE "$HD_ZERO" "$HD_TMP"/journal-*.jsonl; then
    hd_ok "spend is unknown, never zero" "7 perform.ended lines, each spend \"unknown\"; no spend 0"
else
    hd_fail "spend is unknown, never zero" "perform.ended count and non-unknown: '$HD_SPEND'"
fi
# The grep can see a zero: the same line with its spend planted as 0 is caught.
sed 's/\\"spend\\":\\"unknown\\"/\\"spend\\":0/' "$HD_TMP/journal-mixed.jsonl" > "$HD_TMP/zero.jsonl"
if grep -qE "$HD_ZERO" "$HD_TMP/zero.jsonl"; then
    hd_ok "a zero spend would be seen" "the planted \"spend\":0 matches the check above"
else
    hd_fail "a zero spend would be seen" "the check did not match a planted zero"
fi

hd_reset
corpus_gone "$HD_ROOT"
rm -rf "$HD_TMP"
