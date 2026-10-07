# shellcheck shell=bash
# OW-WAR-0148 M13 — optional parts and the tick ladder.
#
# A scratch program, its own throwaway key in its own throwaway ssh-agent,
# and a minimal roadmap; nothing here reads this repository's tickets or
# reaches the caller's agent. Each claim is paired with the refusal that
# shows the control working:
#
#   1. a title-only Warrant gains a test and ticks at observed only when it
#      passes; a failing test refuses the observed tick by name;
#   2. a minimum of observed (a milestone's own, or a type's from profile
#      data) refuses a claimed tick and names the command that reaches it;
#   3. claimed and observed read differently in every view (war show, war
#      tickets, war status, war roadmap, war board, the web page), and a
#      hand-written [signed] nothing backs reads as claimed;
#   4. a KPI's best, latest and target are tracked across three runs;
#   5. a KPI that prints no number is UNKNOWN, never a pass;
#   6. the TaskCompleted hook does nothing on an unrelated task;
#   7. an independent verdict by the performer is refused; another's raises;
#   8. a sign-off needs --ssh-sign, and a signed tick whose text changed no
#      longer reads as signed.

echo "== optional parts and the tick ladder (M13) =="
PLANT_ROOT=$(scratch_corpus TL)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
TL_TMP=$(mktemp -d)
tl_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tl_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
tlw() { "$WAR" --root "$PLANT_ROOT" "$@"; }
tlj() { "$WAR" --root "$PLANT_ROOT" --json "$@" 2>/dev/null; }
tl_py() { python3 -c "import json, sys; v = json.load(sys.stdin); print($1)" 2>/dev/null; }
tl_id() { awk 'NR==1{print $1}' <<<"$1"; }
tl_cl() { cat "$PLANT_ROOT/docs/tickets/$1/atoms/15-checklist.md" 2>/dev/null; }
tl_item() { tlj show "$1" | tl_py "v['result']['items'][$2]['id']"; }

# ---- 1. a title-only Warrant gains a test --------------------------------------
TL_CREATE=$(tlw create "Make the parser fast" 2>&1)
TL_T=$(tl_id "$TL_CREATE")
printf '#!/bin/sh\nexit "${TL_GATE:-1}"\n' > "$PLANT_ROOT/gate.sh"
TL_ADD=$(tlw add "$TL_T" --test "sh gate.sh" --name unit 2>&1); TL_ADD_RC=$?
tlw claim "$TL_T" >/dev/null 2>&1
TL_FAIL=$(tlw done "$TL_T" --check 2>&1); TL_FAIL_RC=$?
TL_CL_AFTER_FAIL=$(tl_cl "$TL_T")
TL_OK=$(TL_GATE=0 tlw done "$TL_T" --check 2>&1); TL_OK_RC=$?
TL_CL_AFTER_OK=$(tl_cl "$TL_T")
TL_JOURNAL=$(cat "$PLANT_ROOT/docs/tickets/$TL_T/journal.jsonl" 2>/dev/null)
if [[ $TL_ADD_RC -eq 0 && $TL_OK_RC -eq 0 ]] && grep -q '\[observed\]' <<<"$TL_CL_AFTER_OK" \
    && grep -q 'ticked at observed: unit passed' <<<"$TL_OK" \
    && grep -q 'stdout_sha256' <<<"$TL_JOURNAL" \
    && [[ -f "$PLANT_ROOT/docs/tickets/$TL_T/atoms/30-checks.md" ]]; then
    tl_ok "a title-only Warrant gains a test" "$TL_T: --test added, ticked at observed, receipt journalled"
else
    tl_fail "a title-only Warrant gains a test" "add $TL_ADD_RC, done $TL_OK_RC: $(head -2 <<<"$TL_OK" | tr '\n' '|') $(grep -c . <<<"$TL_CL_AFTER_OK")"
fi
if [[ $TL_FAIL_RC -eq 2 ]] && grep -qF 'ticket.check-failed' <<<"$TL_FAIL" \
    && grep -qF 'unit (`sh gate.sh`) exited non-zero' <<<"$TL_FAIL" \
    && ! grep -q '\[x\]' <<<"$TL_CL_AFTER_FAIL"; then
    tl_ok "a failing test refuses observed" "ticket.check-failed names unit; nothing ticked"
else
    tl_fail "a failing test refuses observed" "exit $TL_FAIL_RC: $(head -c 300 <<<"$TL_FAIL")"
fi
# The create hint: one line, by default; `[warrants] hints = false` hides it.
TL_HINTS=$(grep -c 'hint (optional): `war add' <<<"$TL_CREATE")
printf '\n[warrants]\nhints = false\n' >> "$PLANT_ROOT/openwarrant.toml"
TL_QUIET=$(tlw create "Quiet one" 2>&1)
if [[ "$TL_HINTS" == 1 ]] && ! grep -q 'hint' <<<"$TL_QUIET"; then
    tl_ok "the create hint, and its switch" "one line by default; none with [warrants] hints = false"
else
    tl_fail "the create hint, and its switch" "hint lines $TL_HINTS; with hints = false: $(tail -1 <<<"$TL_QUIET")"
fi

# ---- 2. a minimum refuses a lower tick ---------------------------------------
TL_R=$(tl_id "$(tlw create "Release" 2>&1)")
TL_M=$(tlj add "$TL_R" --milestone "Beta ships" --min observed | tl_py "v['result']['item']")
tlw claim "$TL_R/$TL_M" >/dev/null 2>&1
TL_LOW=$(tlw done "$TL_R/$TL_M" 2>&1); TL_LOW_RC=$?
TL_LOW_CL=$(tl_cl "$TL_R")
tlw add "$TL_R/$TL_M" --test "true" >/dev/null 2>&1
TL_UP=$(tlw done "$TL_R/$TL_M" --check 2>&1); TL_UP_RC=$?
if [[ $TL_LOW_RC -eq 2 ]] && grep -qF 'ticket.tick-below-minimum' <<<"$TL_LOW" \
    && grep -qF "\`war done $TL_R/$TL_M --check\`" <<<"$TL_LOW" && ! grep -q '\[x\]' <<<"$TL_LOW_CL"; then
    tl_ok "min observed refuses a claimed tick" "ticket.tick-below-minimum, naming war done --check"
else
    tl_fail "min observed refuses a claimed tick" "exit $TL_LOW_RC: $(head -c 300 <<<"$TL_LOW")"
fi
if [[ $TL_UP_RC -eq 0 ]] && grep -q "($TL_M) — done by claude, [0-9-]* \[observed\]" <<<"$(tl_cl "$TL_R")"; then
    tl_ok "--check reaches the minimum" "$TL_R/$TL_M ticked at observed"
else
    tl_fail "--check reaches the minimum" "exit $TL_UP_RC: $(head -c 300 <<<"$TL_UP")"
fi
# A type's minimum, from profile data: a bug's milestones need observed.
mkdir -p "$PLANT_ROOT/profiles"
cp "$REPO_ROOT/profiles/ticket.toml" "$PLANT_ROOT/profiles/ticket.toml"
printf '\n[ticks.types]\nbug = "observed"\n' >> "$PLANT_ROOT/profiles/ticket.toml"
TL_B=$(tl_id "$(tlw create "A crash" --type bug 2>&1)")
TL_BM=$(tlj add "$TL_B" --milestone "Fixed" | tl_py "v['result']['item']")
TL_C=$(tl_id "$(tlw create "A chore" --type chore 2>&1)")
TL_CM=$(tlj add "$TL_C" --milestone "Swept" | tl_py "v['result']['item']")
tlw claim "$TL_B/$TL_BM" >/dev/null 2>&1; tlw claim "$TL_C/$TL_CM" >/dev/null 2>&1
TL_BUG=$(tlw done "$TL_B/$TL_BM" 2>&1); TL_BUG_RC=$?
TL_CHORE=$(tlw done "$TL_C/$TL_CM" 2>&1); TL_CHORE_RC=$?
printf '\n[ticks]\nitem = "verified"\n' >> "$PLANT_ROOT/profiles/ticket.toml"
TL_BADP=$(tlw tickets 2>&1); TL_BADP_RC=$?
command cp "$REPO_ROOT/profiles/ticket.toml" "$PLANT_ROOT/profiles/ticket.toml"
if [[ $TL_BUG_RC -eq 2 && $TL_CHORE_RC -eq 0 ]] && grep -qF 'the minimum the ticket'"'"'s type sets' <<<"$TL_BUG"; then
    tl_ok "a type's minimum is profile data" "bug's milestone refused at claimed; a chore's ticks"
else
    tl_fail "a type's minimum is profile data" "bug exit $TL_BUG_RC: $(head -c 200 <<<"$TL_BUG"); chore exit $TL_CHORE_RC"
fi
if [[ $TL_BADP_RC -ne 0 ]] && grep -qF 'profile.ticks' <<<"$TL_BADP$(tlj check | tl_py "' '.join(d['rule'] for d in v['diagnostics'])")"; then
    tl_ok "a level outside the ladder refused" "profile.ticks"
else
    # The profile is read when the store opens; any refusal must name it.
    if grep -qF '[ticks]' <<<"$TL_BADP"; then
        tl_ok "a level outside the ladder refused" "[ticks] named: $(head -c 120 <<<"$TL_BADP")"
    else
        tl_fail "a level outside the ladder refused" "exit $TL_BADP_RC: $(head -c 200 <<<"$TL_BADP")"
    fi
fi

# ---- 3. claimed and observed read differently, everywhere ---------------------
TL_W=$(tl_id "$(tlw create "Two items" -i plain -i checked 2>&1)")
TL_WP=$(tl_item "$TL_W" 0); TL_WC=$(tl_item "$TL_W" 1)
tlw add "$TL_W/$TL_WC" --test "true" >/dev/null 2>&1
tlw claim "$TL_W/$TL_WP" >/dev/null 2>&1; tlw claim "$TL_W/$TL_WC" >/dev/null 2>&1
tlw done "$TL_W/$TL_WP" >/dev/null 2>&1; tlw done "$TL_W/$TL_WC" --check >/dev/null 2>&1
TL_SHOW=$(tlw show "$TL_W" 2>&1)
TL_LS=$(tlw tickets 2>&1)
TL_ST=$(tlw status 2>&1)
TL_BOARD=$(tlw board 2>&1)
TL_PLAIN_LINE=$(grep -F "plain ($TL_WP)" <<<"$TL_SHOW")
if grep -qF -- "- [x] (claimed) plain ($TL_WP) — done by claude" <<<"$TL_SHOW" \
    && grep -qF -- "- [x] (observed) checked ($TL_WC) — done by claude" <<<"$TL_SHOW" \
    && ! grep -qiE 'observed|verified|signed' <<<"$TL_PLAIN_LINE"; then
    tl_ok "war show: claimed vs observed" "(claimed) and (observed); the claimed line names no check"
else
    tl_fail "war show: claimed vs observed" "$(grep -F '[x]' <<<"$TL_SHOW" | tr '\n' '|')"
fi
if line_has -F "$TL_W " -F "[ticks: 1 claimed, 1 observed]" <<<"$TL_LS"; then
    tl_ok "war tickets: the levels" "[ticks: 1 claimed, 1 observed]"
else
    tl_fail "war tickets: the levels" "$(grep -F "$TL_W" <<<"$TL_LS")"
fi
if line_has -F "- $TL_W  Two items:" -F "1 claimed, 1 observed" <<<"$TL_ST" \
    && grep -qF 'a claimed tick is the performer'"'"'s word' <<<"$TL_ST"; then
    tl_ok "war status: the levels" "the ticks block names both"
else
    tl_fail "war status: the levels" "$(sed -n '/## Ticks/,$p' <<<"$TL_ST" | head -8 | tr '\n' '|')"
fi
if line_has -F "- $TL_W  Two items:" -F "1 claimed, 1 observed" <<<"$TL_BOARD" \
    && grep -qF "(observed) $TL_R/$TL_M" <<<"$TL_BOARD" && grep -qF "(claimed) $TL_C/$TL_CM" <<<"$TL_BOARD"; then
    tl_ok "war board: levels and milestones" "observed and claimed milestones apart"
else
    tl_fail "war board: levels and milestones" "$(sed -n '/## Ticks/,$p' <<<"$TL_BOARD" | head -12 | tr '\n' '|')"
fi
# The roadmap view, in a program with a roadmap record.
mkdir -p "$PLANT_ROOT/docs/roadmap/atoms"
cat > "$PLANT_ROOT/docs/roadmap/roadmap.toml" <<'TOML'
schema = "oh.war/roadmap/v1"
uuid = "01a0cd26-0000-7000-8000-0000000000a1"
program = "Plant Corpus TL"
prefix = "TL"

[[atoms]]
ordinal = 20
role = "phases"
path = "atoms/20-phases.yaml"
TOML
cat > "$PLANT_ROOT/docs/roadmap/atoms/20-phases.yaml" <<'YAML'
schema: "oh.war/roadmap-phases/v1"

phases:
  - id: "TL-PHASE-1"
    title: "Adopt"
    exit: "it is adopted"
    depends_on: []
YAML
TL_RM=$(tlw roadmap 2>&1)
if grep -qF -- "- [x] (observed) $TL_R/$TL_M  Beta ships (Release) — min observed, met" <<<"$TL_RM" \
    && grep -qF -- "- [x] (claimed) $TL_C/$TL_CM  Swept (A chore) — min claimed, met" <<<"$TL_RM"; then
    tl_ok "war roadmap: milestone markers" "(observed) and (claimed), each against its minimum"
else
    tl_fail "war roadmap: milestone markers" "$(sed -n '/Milestones/,$p' <<<"$TL_RM" | head -6 | tr '\n' '|')"
fi
# The web page: the same levels in its API, and a style per level.
TL_UOUT=$(mktemp); TL_UERR=$(mktemp)
env -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" ui --port 0 >"$TL_UOUT" 2>"$TL_UERR" &
TL_UPID=$!
for _ in $(seq 1 60); do grep -q 'http://' "$TL_UOUT" && break; sleep 0.25; done
TL_URL=$(grep -o 'http://[^ ]*' "$TL_UOUT" | head -1)
TL_UH=${TL_URL#http://}; TL_UH=${TL_UH%%/*}
TL_UT=${TL_URL#*#t=}; TL_UT=${TL_UT%%&*}
TL_API=$(curl -s -H "Authorization: Bearer $TL_UT" "http://$TL_UH/api/tickets")
TL_PROG=$(curl -s -H "Authorization: Bearer $TL_UT" "http://$TL_UH/api/progress")
TL_JS=$(curl -s "http://$TL_UH/assets/app.js")
kill "$TL_UPID" 2>/dev/null; wait "$TL_UPID" 2>/dev/null
command rm -f "$TL_UOUT" "$TL_UERR"
TL_UI=$(python3 -c '
import json, sys
d = json.loads(sys.argv[1]); p = json.loads(sys.argv[2])
t = [x for x in d["tickets"] if x["ticket"]["id"] == sys.argv[3]][0]
marks = {i["text"]: (i["tick"]["level"], i["tick_marker"]) for i in t["items"] if i.get("tick")}
ms = {m["target"]: m["marker"] for m in p.get("milestones", [])}
print(marks["plain"][0], marks["plain"][1], marks["checked"][0], marks["checked"][1], ms.get(sys.argv[4], "-"))
' "$TL_API" "$TL_PROG" "$TL_W" "$TL_R/$TL_M" 2>&1)
if [[ "$TL_UI" == "claimed (claimed) observed (observed) (observed)" ]] \
    && grep -qF 'tick-claimed { color:var(--muted); border-style:dashed; }' "$REPO_ROOT/crates/openwarrant-cli/src/webui/assets/app.css" \
    && grep -qF '"badge tick-" + t.level' <<<"$TL_JS"; then
    tl_ok "the web page: levels apart" "API: $TL_UI; claimed is drawn muted and dashed"
else
    tl_fail "the web page: levels apart" "$TL_UI | $(head -c 120 <<<"$TL_API")"
fi
# Refusal: a [signed] marker written by hand, with nothing behind it.
TL_CLF="$PLANT_ROOT/docs/tickets/$TL_W/atoms/15-checklist.md"
sed -i "s/^\(- \[x\] plain ($TL_WP) — done by claude, [0-9-]*\)$/\1 [signed]/" "$TL_CLF"
assert_present '[signed]' "$TL_CLF"
TL_SHOWF=$(tlw show "$TL_W" 2>&1)
TL_LSF=$(tlw tickets 2>&1)
if grep -qF -- "- [x] (claimed) plain ($TL_WP)" <<<"$TL_SHOWF" && grep -qF 'its [signed] marker is not believed' <<<"$TL_SHOWF" \
    && line_has -F "$TL_W " -F "[ticks: 1 claimed, 1 observed]" <<<"$TL_LSF"; then
    tl_ok "an unbacked [signed] reads claimed" "show: (claimed), not believed; tickets: still 1 claimed"
else
    tl_fail "an unbacked [signed] reads claimed" "$(grep -F "plain" <<<"$TL_SHOWF")"
fi

# ---- 4. a KPI's best, latest and target across three runs ---------------------
TL_K=$(tl_id "$(tlw create "Latency" 2>&1)")
printf '#!/bin/sh\necho "$TL_P95"\n' > "$PLANT_ROOT/p95.sh"
tlw add "$TL_K" --kpi p95 --cmd "sh p95.sh" --direction min --target 100 >/dev/null 2>&1
TL_K1=$(TL_P95=120 tlw kpi run "$TL_K" 2>&1)
TL_K2=$(TL_P95=90 tlw kpi run "$TL_K" 2>&1)
TL_K3=$(TL_P95=99 tlw kpi run "$TL_K" 2>&1)
TL_KS=$(tlj show "$TL_K" | tl_py "(lambda k: '%s %s %s %s %s' % (k['best'], k['latest'], k['target'], k['runs'], k['latest_verdict']))(v['result']['checks']['kpis'][0])")
TL_KJ=$(grep -c '"ticket.kpi_run"' "$PLANT_ROOT/docs/tickets/$TL_K/journal.jsonl")
if [[ "$TL_KS" == "90.0 99.0 100.0 3 pass" && "$TL_KJ" == 3 ]] && grep -qF 'best 90, target <= 100, 3 runs' <<<"$TL_K3"; then
    tl_ok "KPI best, latest, target, 3 runs" "best 90, latest 99, target 100; 3 runs journalled"
else
    tl_fail "KPI best, latest, target, 3 runs" "$TL_KS; journal $TL_KJ; $(tail -1 <<<"$TL_K3")"
fi
if grep -qF 'kpi.target-missed' <<<"$TL_K1" && grep -qF 'latest 120 (fail)' <<<"$TL_K1"; then
    tl_ok "a run past its target is a miss" "120 against <= 100: fail, said by rule"
else
    tl_fail "a run past its target is a miss" "$(head -c 300 <<<"$TL_K1")"
fi

# ---- 5. a KPI that prints no number is UNKNOWN --------------------------------
tlw add "$TL_K" --kpi words --cmd "echo fast" --direction max --target 1 >/dev/null 2>&1
TL_U=$(TL_P95=80 tlj kpi run "$TL_K")
TL_UV=$(tl_py "(v['exit_code'], [(r['name'], r['verdict'], r.get('value')) for r in v['result']['runs']], [d['rule'] for d in v['diagnostics']])" <<<"$TL_U")
if [[ "$TL_UV" == "(2, [('p95', 'pass', 80.0), ('words', 'unknown', None)], ['kpi.unknown'])" ]]; then
    tl_ok "no number is UNKNOWN" "words: unknown, no value; p95 beside it: 80, pass"
else
    tl_fail "no number is UNKNOWN" "$TL_UV"
fi
tlw claim "$TL_K" >/dev/null 2>&1
TL_UD=$(TL_P95=80 tlw done "$TL_K" --check 2>&1); TL_UD_RC=$?
if [[ $TL_UD_RC -eq 2 ]] && grep -qF 'UNKNOWN (ticket.check-unknown)' <<<"$TL_UD" && ! grep -q '\[x\]' <<<"$(tl_cl "$TL_K")"; then
    tl_ok "UNKNOWN holds an observed tick" "ticket.check-unknown; nothing ticked"
else
    tl_fail "UNKNOWN holds an observed tick" "exit $TL_UD_RC: $(head -c 300 <<<"$TL_UD")"
fi

# ---- 6. the TaskCompleted hook does nothing on an unrelated task ---------------
TL_H=$(tl_id "$(tlw create "Hooked" -i "the step" 2>&1)")
TL_HI=$(tl_item "$TL_H" 0)
TL_HSUM0=$(sha256sum < "$PLANT_ROOT/docs/tickets/$TL_H/atoms/15-checklist.md")
tl_hook() {
    printf '{"hook_event_name":"TaskCompleted","cwd":"%s","task_id":"task-1","task_subject":"%s"}' "$PLANT_ROOT" "$1" \
        | PATH="$REPO_ROOT/target/debug:$PATH" bash "$REPO_ROOT/.claude/hooks/task-completed.sh" 2>"$TL_TMP/hook.err"
}
TL_HOUT=$(tl_hook "Implement user authentication"); TL_HRC=$?
TL_HSUM1=$(sha256sum < "$PLANT_ROOT/docs/tickets/$TL_H/atoms/15-checklist.md")
if [[ $TL_HRC -eq 0 && -z "$TL_HOUT" && ! -s "$TL_TMP/hook.err" && "$TL_HSUM0" == "$TL_HSUM1" ]]; then
    tl_ok "the hook ignores an unrelated task" "exit 0, no output, the checklist byte-identical"
else
    tl_fail "the hook ignores an unrelated task" "exit $TL_HRC, out '$TL_HOUT', err '$(head -c 120 "$TL_TMP/hook.err")'"
fi
TL_HOUT=$(tl_hook "Did $TL_H/$TL_HI"); TL_HRC=$?
if [[ $TL_HRC -eq 0 && -z "$TL_HOUT" ]] && grep -q "^- \[x\] the step ($TL_HI) — done by claude" <<<"$(tl_cl "$TL_H")"; then
    tl_ok "the hook ticks the task's item" "exit 0, nothing on stdout, $TL_H/$TL_HI ticked"
else
    tl_fail "the hook ticks the task's item" "exit $TL_HRC: $(head -c 200 "$TL_TMP/hook.err")"
fi
# A failing check never holds the task: still exit 0, said on stderr.
TL_H2=$(tl_id "$(tlw create "Hooked again" -i "red" 2>&1)")
TL_H2I=$(tl_item "$TL_H2" 0)
tlw add "$TL_H2/$TL_H2I" --test "false" >/dev/null 2>&1
TL_HOUT=$(tl_hook "Did $TL_H2/$TL_H2I"); TL_HRC=$?
if [[ $TL_HRC -eq 0 && -z "$TL_HOUT" ]] && grep -qF 'bridge.not-ticked' "$TL_TMP/hook.err" \
    && ! grep -q '\[x\]' <<<"$(tl_cl "$TL_H2")"; then
    tl_ok "a failing check never holds a task" "exit 0; not ticked, said on stderr"
else
    tl_fail "a failing check never holds a task" "exit $TL_HRC: $(head -c 200 "$TL_TMP/hook.err")"
fi
# The task-list bridge: proposes, and writes only with --apply.
mkdir -p "$TL_TMP/tasks"
TL_H3=$(tl_id "$(tlw create "Listed" -i "one" -i "two" 2>&1)")
TL_H3A=$(tl_item "$TL_H3" 0); TL_H3B=$(tl_item "$TL_H3" 1)
printf '{"id":"1","subject":"Do %s/%s","status":"completed"}\n' "$TL_H3" "$TL_H3A" > "$TL_TMP/tasks/1.json"
printf '{"id":"2","subject":"Do %s/%s","status":"in_progress"}\n' "$TL_H3" "$TL_H3B" > "$TL_TMP/tasks/2.json"
printf 'not json\n' > "$TL_TMP/tasks/3.json"
TL_BR=$(tlw bridge claude-tasks --dir "$TL_TMP/tasks" 2>&1)
TL_BR_CL=$(tl_cl "$TL_H3")
TL_BRA=$(tlw bridge claude-tasks --dir "$TL_TMP/tasks" --apply 2>&1)
if grep -qF "would run war done $TL_H3/$TL_H3A" <<<"$TL_BR" && grep -qF 'bridge.task-unreadable' <<<"$TL_BR" \
    && ! grep -q '\[x\]' <<<"$TL_BR_CL" \
    && grep -q "^- \[x\] one ($TL_H3A)" <<<"$(tl_cl "$TL_H3")" && grep -q "^- \[ \] two ($TL_H3B)$" <<<"$(tl_cl "$TL_H3")"; then
    tl_ok "the task-list bridge" "proposes the completed one; --apply ticks it; in-progress and unreadable left"
else
    tl_fail "the task-list bridge" "$(head -c 300 <<<"$TL_BR") | $(head -c 200 <<<"$TL_BRA")"
fi

# ---- 7. independent: the performer is refused, another raises ------------------
TL_V=$(tl_id "$(tlw create "Reviewable" -i "the change" 2>&1)")
TL_VI=$(tl_item "$TL_V" 0)
tlw claim "$TL_V/$TL_VI" >/dev/null 2>&1; tlw done "$TL_V/$TL_VI" >/dev/null 2>&1
tl_resp() {
    cat > "$TL_TMP/resp.toml" <<TOML
schema = "oh.war/tick-verification-response/v1"
ticket = "$TL_V"
item = "$TL_VI"
[verification]
obligation = "$TL_V/$TL_VI"
performer = "claude"
disposition = "established"
evidence = "read the diff and ran the tests"
[verification.verifier]
actor = "$1"
kind = "agent"
[verification.verifier.independence]
performer_transcript_blind = true
performer_rationale_blind = true
separate_writable_workspace = true
cannot_modify_subject_artifacts = true
cannot_modify_gate_definition = true
cannot_modify_gate_fixtures = true
separate_context_compilation = true
distinct_model_required = true
distinct_human_required = false
TOML
}
tl_resp claude
TL_SELF=$(tlw verify "$TL_V/$TL_VI" --response "$TL_TMP/resp.toml" 2>&1); TL_SELF_RC=$?
tl_resp reviewer-b
TL_IND=$(tlw verify "$TL_V/$TL_VI" --response "$TL_TMP/resp.toml" 2>&1); TL_IND_RC=$?
if [[ $TL_SELF_RC -eq 2 && $TL_IND_RC -eq 0 ]] && grep -qF 'tick.self-verification' <<<"$TL_SELF" \
    && grep -qF -- "- [x] (independent: verified by reviewer-b) the change" <<<"$(tlw show "$TL_V" 2>&1)"; then
    tl_ok "independent: not the performer" "self refused by rule; reviewer-b raises it"
else
    tl_fail "independent: not the performer" "self $TL_SELF_RC: $(head -c 150 <<<"$TL_SELF") | $TL_IND_RC: $(head -c 150 <<<"$TL_IND")"
fi

# ---- 8. signed: a human's key, in a throwaway agent ----------------------------
ssh-keygen -q -t ed25519 -N "" -C plant -f "$TL_TMP/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$TL_TMP/id_plant.pub")" \
    > "$PLANT_ROOT/docs/authority/allowed_signers"
cat > "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/110-tick-ladder.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "plant"
ROLES
TL_OLD_SOCK=${SSH_AUTH_SOCK:-}
eval "$(ssh-agent -s)" >/dev/null
ssh-add -q "$TL_TMP/id_plant" 2>/dev/null
TL_KEYS=$(ssh-add -l 2>/dev/null | grep -c .)
if [[ "$TL_KEYS" != 1 ]]; then
    printf 'PLANT SETUP FAILED: the throwaway agent holds %s keys, not 1\n' "$TL_KEYS" >&2
    ssh-agent -k >/dev/null 2>&1; exit 9
fi
TL_S=$(tl_id "$(tlw create "Signed off" -i "the release" 2>&1)")
TL_SI=$(tl_item "$TL_S" 0)
tlw claim "$TL_S/$TL_SI" >/dev/null 2>&1; tlw done "$TL_S/$TL_SI" >/dev/null 2>&1
TL_NOSSH=$(tlw sign "$TL_S/$TL_SI" --as "Plant Signer" 2>&1); TL_NOSSH_RC=$?
TL_SIGN=$(tlw sign "$TL_S/$TL_SI" --ssh-sign --as "Plant Signer" 2>&1); TL_SIGN_RC=$?
TL_SSHOW=$(tlw show "$TL_S" 2>&1)
if [[ $TL_NOSSH_RC -eq 2 && $TL_SIGN_RC -eq 0 ]] && grep -qF 'sign.ssh-required' <<<"$TL_NOSSH" \
    && grep -qF -- "- [x] (signed: signed off by Plant Signer) the release" <<<"$TL_SSHOW" \
    && [[ -f "$PLANT_ROOT/docs/authority/responses/$TL_S--$TL_SI.signoff.response.toml.sig" ]]; then
    tl_ok "signed: a verified sign-off" "without --ssh-sign refused; with it, signed by Plant Signer"
else
    tl_fail "signed: a verified sign-off" "$TL_NOSSH_RC/$TL_SIGN_RC: $(head -c 200 <<<"$TL_SIGN") | $(grep -F 'the release' <<<"$TL_SSHOW")"
fi
# The sign-off is over the item's words: reword it, and it no longer reads signed.
sed -i "s/^- \[x\] the release ($TL_SI)/- [x] the release, reworded ($TL_SI)/" "$PLANT_ROOT/docs/tickets/$TL_S/atoms/15-checklist.md"
TL_SSHOW2=$(tlw show "$TL_S" 2>&1)
if grep -qF -- "- [x] (claimed) the release, reworded ($TL_SI)" <<<"$TL_SSHOW2" \
    && grep -qF 'text changed since Plant Signer signed it off' <<<"$TL_SSHOW2"; then
    tl_ok "a reworded item is not signed" "reads (claimed): the text changed since the sign-off"
else
    tl_fail "a reworded item is not signed" "$(grep -F 'reworded' <<<"$TL_SSHOW2")"
fi
ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$TL_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$TL_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
unset SSH_AGENT_PID

command rm -rf "$TL_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT TL_TMP TL_CREATE TL_T TL_ADD TL_ADD_RC TL_FAIL TL_FAIL_RC TL_CL_AFTER_FAIL TL_OK TL_OK_RC \
    TL_CL_AFTER_OK TL_JOURNAL TL_HINTS TL_QUIET TL_R TL_M TL_LOW TL_LOW_RC TL_LOW_CL TL_UP TL_UP_RC TL_B TL_BM \
    TL_C TL_CM TL_BUG TL_BUG_RC TL_CHORE TL_CHORE_RC TL_BADP TL_BADP_RC TL_W TL_WP TL_WC TL_SHOW TL_LS TL_ST \
    TL_BOARD TL_PLAIN_LINE TL_RM TL_UOUT TL_UERR TL_UPID TL_URL TL_UH TL_UT TL_API TL_PROG TL_JS TL_UI TL_CLF \
    TL_SHOWF TL_LSF TL_K TL_K1 TL_K2 TL_K3 TL_KS TL_KJ TL_U TL_UV TL_UD TL_UD_RC TL_H TL_HI TL_HSUM0 TL_HSUM1 \
    TL_HOUT TL_HRC TL_H2 TL_H2I TL_H3 TL_H3A TL_H3B TL_BR TL_BR_CL TL_BRA TL_V TL_VI TL_SELF TL_SELF_RC TL_IND \
    TL_IND_RC TL_OLD_SOCK TL_KEYS TL_S TL_SI TL_NOSSH TL_NOSSH_RC TL_SIGN TL_SIGN_RC TL_SSHOW TL_SSHOW2
unset -f tl_ok tl_fail tlw tlj tl_py tl_id tl_cl tl_item tl_hook tl_resp
