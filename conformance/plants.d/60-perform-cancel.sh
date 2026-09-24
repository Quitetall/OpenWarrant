# shellcheck shell=bash
# OW-WAR-0131: `war perform`'s cancellation, writer handoff, OS admission, and
# the claude adapter.
#
# Every case runs on a scratch program, never this corpus: PC-WAR-0002 there
# has two agent stages, and its performer is writes-until-killed.sh, which
# appends a line to a marker file every 100 ms and starts a child that does the
# same. A marker that keeps growing is a writer still alive; one that stops is
# a writer gone. The lines carry pids, so "gone" is also asked of the process
# table, and anything the control under test failed to kill is killed here.
#
# Waits are polls with generous bounds, so a loaded machine is slow rather
# than wrong. The one fixed interval is OBL-001's own: the marker is unchanged
# one second after `war` has exited.

PC_TMP=$(mktemp -d)
PC_ROOT=$(scratch_corpus PC)
# Sourced outside the battery, `scratch_corpus` is undefined and every
# `git -C "$PC_ROOT"` below would act on this repository. Refuse.
[[ -d "${PC_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
PC_W=PC-WAR-0002
PC_DIR="$PC_ROOT/docs/warrants/$PC_W"
PC_FX="$REPO_ROOT/conformance/fixtures/performer/writes-until-killed.sh"
PC_ADAPTER="$REPO_ROOT/tools/performer/claude-performer.sh"
PC_WAR="$REPO_ROOT/${WAR#./}"

pc_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
pc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

"$PC_WAR" --root "$PC_ROOT" new "Cancellation plant" > /dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: war new in %s\n' "$PC_ROOT" >&2; exit 9; }
[[ -d "$PC_DIR" ]] || { printf 'PLANT SETUP FAILED: %s was not created\n' "$PC_W" >&2; exit 9; }
# The scaffold's adopt Warrant has agent stages of its own; none of them is
# this plant's, and none should be performed by it.
sed -i 's/executor_kind: "agent"/executor_kind: "human"/' "$PC_ROOT/docs/warrants/PC-WAR-0001/atoms/45-milestones.yaml"
cat > "$PC_DIR/atoms/45-milestones.yaml" << 'YAML'
schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "cancellation plant"
    stage_refs: ["STAGE-001", "STAGE-002"]

stages:
  - id: "STAGE-001"
    title: "a stage whose performer writes until it is killed"
    executor_kind: "agent"
    responsibility_tier: "T2"
    executor_ref: "agent://fixture"
  - id: "STAGE-002"
    title: "a stage the claude adapter performs"
    executor_kind: "agent"
    responsibility_tier: "T2"
    executor_ref: "agent://fixture"
YAML
printf '\n[perform]\nperformer_argv = ["%s"]\nperformer_timeout_secs = 600\nmax_concurrent = 1\n' "$PC_FX" >> "$PC_ROOT/openwarrant.toml"
"$PC_WAR" --root "$PC_ROOT" compile > /dev/null 2>&1
git -C "$PC_ROOT" add -A > /dev/null 2>&1
git -C "$PC_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "cancellation plant" > /dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: commit in %s\n' "$PC_ROOT" >&2; exit 9; }
assert_present "$PC_FX" "$PC_ROOT/openwarrant.toml"
assert_present 'id: "STAGE-002"' "$PC_DIR/atoms/45-milestones.yaml"

# --- helpers -----------------------------------------------------------------

# Alive = in the process table and not a zombie. A zombie cannot write, and an
# orphan's zombie may sit unreaped under a subreaper for a while.
pc_alive() {
    local s
    s=$(ps -o stat= -p "$1" 2> /dev/null) || return 1
    [[ -n "$s" && "$s" != Z* ]]
}
pc_lines() { grep -c "^$2 " "$1" 2> /dev/null || true; }
pc_pids() { awk '{print $2}' "$1" 2> /dev/null | sort -u; }
# Until both the performer and its child have written, at most $2 seconds.
pc_wait_writing() {
    local marker="$1" i
    for ((i = 0; i < ${2:-60} * 10; i++)); do
        [[ "$(pc_lines "$marker" performer)" -ge 1 && "$(pc_lines "$marker" child)" -ge 1 ]] && return 0
        sleep 0.1
    done
    return 1
}
# Until process $1 has exited, at most $2 seconds.
pc_wait_exit() {
    local i
    for ((i = 0; i < ${2:-60} * 10; i++)); do
        pc_alive "$1" || return 0
        sleep 0.1
    done
    return 1
}
# Until file $1 exists (or, with "gone", does not), at most $3 seconds.
pc_wait_file() {
    local i
    for ((i = 0; i < ${3:-60} * 10; i++)); do
        if [[ "$2" == gone ]]; then [[ -e "$1" ]] || return 0; else [[ -e "$1" ]] && return 0; fi
        sleep 0.1
    done
    return 1
}
# The check OBL-001 rests on. Quiet (0) when the marker's performer and child
# counts are unchanged across one second AND no pid in it is alive; otherwise
# 1, with what it saw in PC_SEEN. The negative control below shows it can
# return 1.
pc_quiet() {
    local marker="$1" p0 c0 p1 c1 pid live=""
    p0=$(pc_lines "$marker" performer)
    c0=$(pc_lines "$marker" child)
    sleep 1
    p1=$(pc_lines "$marker" performer)
    c1=$(pc_lines "$marker" child)
    while read -r pid; do
        [[ -n "$pid" ]] && pc_alive "$pid" && live="$live $pid"
    done < <(pc_pids "$marker")
    PC_SEEN="performer $p0->$p1, child $c0->$c1, alive:${live:- none}"
    [[ "$p0" -ge 1 && "$c0" -ge 1 && "$p0" == "$p1" && "$c0" == "$c1" && -z "$live" ]]
}
# Kill every writer a marker names, and its group — never this shell's own.
pc_kill_writers() {
    local pid pg mine
    mine=$(ps -o pgid= -p $$ | tr -d ' ')
    while read -r pid; do
        [[ -n "$pid" ]] || continue
        pg=$(ps -o pgid= -p "$pid" 2> /dev/null | tr -d ' ')
        [[ -n "$pg" && "$pg" != "$mine" && "$pg" -gt 1 ]] && kill -KILL -- "-$pg" 2> /dev/null
        kill -KILL "$pid" 2> /dev/null
    done < <(pc_pids "$1")
    return 0
}
pc_compiled() { grep -c '"type":"dispatch.compiled"' "$PC_DIR/journal.jsonl" 2> /dev/null || true; }
pc_packets() { find "$PC_DIR/dispatches" -name '*.json' ! -name 'answer-*' 2> /dev/null | wc -l | tr -d ' '; }
pc_submissions() { find "$PC_DIR/submissions" -name '*.json' 2> /dev/null | wc -l | tr -d ' '; }
pc_writers() { find "$PC_DIR/dispatches" -name '*.writer' 2> /dev/null | wc -l | tr -d ' '; }
pc_reset() { corpus_reset "$PC_ROOT"; }
# Start `war perform <stage>` in the background with marker $2; sets PC_PID.
pc_start() {
    WRITES_UNTIL_KILLED_MARKER="$2" "$PC_WAR" --root "$PC_ROOT" perform "$PC_W" "$1" --prototype > "$3" 2>&1 &
    PC_PID=$!
}
# Signal `war`, and collect its exit status into PC_STATUS.
pc_stop() {
    kill "-$2" "$1" 2> /dev/null
    if pc_wait_exit "$1" 60; then
        wait "$1"
        PC_STATUS=$?
    else
        kill -KILL "$1" 2> /dev/null
        wait "$1" 2> /dev/null
        PC_STATUS=timeout
    fi
}

PC_ALL_OUT="$PC_TMP/all-outputs"
: > "$PC_ALL_OUT"

# --- OBL-001: SIGINT and SIGTERM to `war` leave no writer behind ------------

for PC_SIG in INT TERM; do
    PC_NAME="SIG$PC_SIG cancels performer+child"
    pc_reset
    PC_MARK="$PC_TMP/cancel-$PC_SIG.marker"
    PC_OUT="$PC_TMP/cancel-$PC_SIG.out"
    rm -f "$PC_MARK"
    pc_start STAGE-001 "$PC_MARK" "$PC_OUT"
    if ! pc_wait_writing "$PC_MARK" 60; then
        pc_stop "$PC_PID" KILL
        pc_kill_writers "$PC_MARK"
        pc_fail "$PC_NAME" "the performer never started writing: $(head -3 "$PC_OUT" | tr '\n' ' ')"
        continue
    fi
    PC_WRITER_SEEN=$(pc_writers)
    pc_stop "$PC_PID" "$PC_SIG"
    cat "$PC_OUT" >> "$PC_ALL_OUT"
    if pc_quiet "$PC_MARK"; then PC_Q=quiet; else PC_Q=orphan; fi
    if [[ "$PC_STATUS" != 0 && "$PC_STATUS" != timeout ]] \
        && grep -q "perform.cancelled .*SIG$PC_SIG received" "$PC_OUT" \
        && [[ "$PC_Q" == quiet ]] \
        && [[ "$(pc_submissions)" == 0 ]] && [[ "$(pc_packets)" == 1 ]] \
        && [[ "$PC_WRITER_SEEN" == 1 ]] && [[ "$(pc_writers)" == 0 ]]; then
        pc_ok "$PC_NAME" "rejected by perform.cancelled (exit $PC_STATUS); $PC_SEEN; no submission, Dispatch kept, record gone"
    else
        pc_fail "$PC_NAME" "exit $PC_STATUS; $PC_Q ($PC_SEEN); submissions $(pc_submissions), packets $(pc_packets), writer records $PC_WRITER_SEEN->$(pc_writers); $(grep -m1 -E 'ERROR|PASS' "$PC_OUT")"
    fi
    pc_kill_writers "$PC_MARK"
done

# The negative control: the same fixture, put in its own session by a plain
# shell (`setsid`) whose parent is then killed. Nothing kills its group, so the
# check above must call it an orphan — or the check is blind.
PC_MARK="$PC_TMP/orphan.marker"
rm -f "$PC_MARK"
WRITES_UNTIL_KILLED_MARKER="$PC_MARK" bash -c 'setsid "$1" < /dev/null > /dev/null 2>&1 & wait' _ "$PC_FX" &
PC_PARENT=$!
if pc_wait_writing "$PC_MARK" 60; then
    pc_stop "$PC_PARENT" TERM
    if pc_quiet "$PC_MARK"; then
        pc_fail "the check sees an orphan" "a setsid writer whose parent died was reported quiet: $PC_SEEN"
    else
        pc_ok "the check sees an orphan" "negative control: setsid writer reported orphan ($PC_SEEN)"
    fi
else
    pc_stop "$PC_PARENT" KILL
    pc_fail "the check sees an orphan" "the setsid fixture never started writing"
fi
pc_kill_writers "$PC_MARK"

# No switch in the shipped binary turns the handler off: `war perform` reads
# no environment variable at all.
if grep -nE 'env::var|var_os\(' crates/openwarrant-cli/src/perform.rs > /dev/null; then
    pc_fail "no switch disables the handler" "$(grep -nE 'env::var|var_os\(' crates/openwarrant-cli/src/perform.rs | head -1)"
elif grep -q 'register_usize(sig' crates/openwarrant-cli/src/perform.rs; then
    pc_ok "no switch disables the handler" "perform.rs reads no environment; the handler is unconditional"
else
    pc_fail "no switch disables the handler" "perform.rs registers no SIGINT/SIGTERM flag"
fi

# --- OBL-002: a second writer is refused while the first may still write ----

# A first performer running; the second call on the same stage is refused,
# starts nothing, and compiles nothing.
pc_reset
PC_MARK="$PC_TMP/first.marker"
PC_MARK2="$PC_TMP/second.marker"
rm -f "$PC_MARK" "$PC_MARK2"
pc_start STAGE-001 "$PC_MARK" "$PC_TMP/first.out"
if pc_wait_writing "$PC_MARK" 60; then
    PC_FIRST=$(awk '$1 == "performer" {print $2; exit}' "$PC_MARK")
    PC_J0=$(pc_compiled)
    PC_P0=$(pc_packets)
    PC_OUT=$(WRITES_UNTIL_KILLED_MARKER="$PC_MARK2" "$PC_WAR" --root "$PC_ROOT" perform "$PC_W" STAGE-001 --prototype 2>&1)
    PC_STATUS=$?
    echo "$PC_OUT" >> "$PC_ALL_OUT"
    sleep 0.5 # a second performer, had one started, would have written by now
    if [[ $PC_STATUS -ne 0 ]] && grep -q "perform.writer-alive .*by pid $PC_FIRST " <<< "$PC_OUT" \
        && [[ ! -e "$PC_MARK2" ]] && [[ "$(pc_compiled)" == "$PC_J0" ]] && [[ "$(pc_packets)" == "$PC_P0" ]] \
        && ! grep -q 'perform.writer-unknown' <<< "$PC_OUT"; then
        pc_ok "a second writer is refused" "rejected by perform.writer-alive (pid $PC_FIRST); no marker, no new Dispatch"
    else
        pc_fail "a second writer is refused" "exit $PC_STATUS; second marker $([[ -e "$PC_MARK2" ]] && echo present || echo absent); compiled $PC_J0->$(pc_compiled); $(grep -m1 -E 'ERROR|UNKNOWN|PASS' <<< "$PC_OUT")"
    fi
    pc_stop "$PC_PID" TERM
    cat "$PC_TMP/first.out" >> "$PC_ALL_OUT"
else
    pc_stop "$PC_PID" KILL
    pc_fail "a second writer is refused" "the first performer never started writing"
fi
pc_kill_writers "$PC_MARK"
pc_kill_writers "$PC_MARK2"

# A record naming a live process whose start time does not match: not shown
# to be the writer, not shown gone — refused as unknown.
pc_reset
PC_MARK="$PC_TMP/unknown.marker"
rm -f "$PC_MARK" "$PC_TMP/impostor.pid"
setsid bash -c 'echo $$ > "$1"; exec sleep 300' _ "$PC_TMP/impostor.pid" < /dev/null > /dev/null 2>&1 &
PC_IMP_JOB=$!
if pc_wait_file "$PC_TMP/impostor.pid" present 30 && PC_IMP=$(cat "$PC_TMP/impostor.pid") && pc_alive "$PC_IMP" \
    && [[ "$(ps -o pgid= -p "$PC_IMP" | tr -d ' ')" == "$PC_IMP" ]]; then
    mkdir -p "$PC_DIR/dispatches"
    printf '{"pid": %s, "pgid": %s, "leader_started": "boot+0ticks", "stage_id": "STAGE-001", "dispatch_id": "PLANTED"}\n' \
        "$PC_IMP" "$PC_IMP" > "$PC_DIR/dispatches/PLANTED.writer"
    assert_present '"leader_started": "boot+0ticks"' "$PC_DIR/dispatches/PLANTED.writer"
    PC_J0=$(pc_compiled)
    PC_OUT=$(WRITES_UNTIL_KILLED_MARKER="$PC_MARK" "$PC_WAR" --root "$PC_ROOT" perform "$PC_W" STAGE-001 --prototype 2>&1)
    PC_STATUS=$?
    echo "$PC_OUT" >> "$PC_ALL_OUT"
    sleep 0.5
    if [[ $PC_STATUS -ne 0 ]] && grep -q 'perform.writer-unknown .*dispatch PLANTED' <<< "$PC_OUT" \
        && grep -q "pid $PC_IMP is alive but started at" <<< "$PC_OUT" \
        && ! grep -q 'perform.writer-alive' <<< "$PC_OUT" \
        && [[ ! -e "$PC_MARK" ]] && [[ "$(pc_compiled)" == "$PC_J0" ]] \
        && [[ -e "$PC_DIR/dispatches/PLANTED.writer" ]]; then
        pc_ok "a mismatched writer is unknown" "rejected by perform.writer-unknown (start time); nothing started, record kept"
    else
        pc_fail "a mismatched writer is unknown" "exit $PC_STATUS; marker $([[ -e "$PC_MARK" ]] && echo present || echo absent); $(grep -m1 -E 'ERROR|UNKNOWN|PASS' <<< "$PC_OUT")"
    fi
else
    pc_fail "a mismatched writer is unknown" "PLANT SETUP: the impostor process did not start as its own group"
fi
{
    [[ -s "$PC_TMP/impostor.pid" ]] && kill -KILL -- "-$(cat "$PC_TMP/impostor.pid")"
    kill -KILL "$PC_IMP_JOB"
    wait "$PC_IMP_JOB"
} 2> /dev/null
pc_kill_writers "$PC_MARK"

# A record naming a group that is gone: the refusal is not blanket. The call
# proceeds, the record is removed, and the performer starts.
pc_reset
PC_MARK="$PC_TMP/gone.marker"
rm -f "$PC_MARK"
PC_DEAD=$(bash -c 'echo $$')
pc_wait_exit "$PC_DEAD" 10
if kill -0 -- "-$PC_DEAD" 2> /dev/null; then
    pc_fail "a gone writer is handed off" "PLANT SETUP: process group $PC_DEAD is not gone"
else
    mkdir -p "$PC_DIR/dispatches"
    printf '{"pid": %s, "pgid": %s, "leader_started": "boot+1ticks", "stage_id": "STAGE-001", "dispatch_id": "PLANTED"}\n' \
        "$PC_DEAD" "$PC_DEAD" > "$PC_DIR/dispatches/PLANTED.writer"
    assert_present "\"pgid\": $PC_DEAD" "$PC_DIR/dispatches/PLANTED.writer"
    pc_start STAGE-001 "$PC_MARK" "$PC_TMP/gone.out"
    if pc_wait_writing "$PC_MARK" 60; then
        PC_REMOVED=$([[ -e "$PC_DIR/dispatches/PLANTED.writer" ]] && echo kept || echo removed)
        pc_stop "$PC_PID" TERM
        cat "$PC_TMP/gone.out" >> "$PC_ALL_OUT"
        if [[ "$PC_REMOVED" == removed ]] \
            && grep -q "perform.writer-stopped .*dispatch PLANTED (pid $PC_DEAD" "$PC_TMP/gone.out" \
            && grep -q 'perform.cancelled' "$PC_TMP/gone.out" \
            && ! grep -qE 'perform.writer-(alive|unknown)' "$PC_TMP/gone.out"; then
            pc_ok "a gone writer is handed off" "perform.writer-stopped; the record was removed and the performer started"
        else
            pc_fail "a gone writer is handed off" "record $PC_REMOVED; $(grep -m1 -E 'ERROR|UNKNOWN|PASS' "$PC_TMP/gone.out")"
        fi
    else
        pc_stop "$PC_PID" KILL
        pc_fail "a gone writer is handed off" "the performer never started: $(head -3 "$PC_TMP/gone.out" | tr '\n' ' ')"
    fi
fi
pc_kill_writers "$PC_MARK"

# --- OBL-003: the supported branch is the compiled one ----------------------

# The refusal branch is exercised by perform.rs's unit test (cargo test); the
# battery holds it to existing. On this runner, every run above got past
# admission, so the group-kill branch is what was compiled.
if grep -q 'fn a_platform_without_the_group_kill_is_refused_before_anything_runs' crates/openwarrant-cli/src/perform.rs \
    && grep -q 'perform.cancelled' "$PC_ALL_OUT" \
    && ! grep -q 'perform.unsupported-os' "$PC_ALL_OUT"; then
    pc_ok "$(uname -s) is admitted" "no perform.unsupported-os on this OS; the refusal is unit-tested"
else
    pc_fail "$(uname -s) is admitted" "$(grep -m1 'perform.unsupported-os' "$PC_ALL_OUT" || echo 'the unit test for the refusal is missing')"
fi

# --- OBL-004: the claude adapter passes the model's answer or nothing -------

# A fake `claude`, first on PATH, that answers from a mode and records its
# argv. A real model is never run: the plant refuses unless `claude` resolves
# to the fake.
mkdir -p "$PC_TMP/bin"
cat > "$PC_TMP/bin/claude" << 'SH'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$FAKE_CLAUDE_ARGV"
exec python3 -c '
import json, os, sys
text = sys.stdin.read()
d = json.JSONDecoder().raw_decode(text[text.index("\n{") + 1:])[0]
mode = os.environ["FAKE_CLAUDE_MODE"]
sub = {"schema": "oh.war/stage-submission/v1", "dispatch_id": d["dispatch_id"],
       "attempt_id": d.get("attempt_id"), "contract_digest": d.get("contract_digest"),
       "stage_id": d.get("stage_id"),
       "claims": [{"id": "C-001", "statement": "The fake claude ran and did nothing else."}],
       "artifact_refs": [], "blockers": [], "requested_next_action": "verify"}
if mode == "other":
    sub["dispatch_id"] = "D-SOMEONE-ELSE"
if mode == "resolve":
    sub["requested_next_action"] = "resolve"
out = "I did the work; it is done.\n" if mode == "notjson" else json.dumps(sub, indent=2) + "\n"
open(os.environ["FAKE_CLAUDE_OUT"], "w").write(out)
sys.stdout.write(out)
sys.exit(1 if mode == "fail" else 0)
'
SH
chmod +x "$PC_TMP/bin/claude"
PC_PATH="$PC_TMP/bin:$PATH"
if [[ "$(PATH="$PC_PATH" command -v claude)" != "$PC_TMP/bin/claude" ]]; then
    printf 'PLANT SETUP FAILED: claude does not resolve to the fake; refusing to run a real model\n' >&2
    exit 9
fi
# The tool list the adapter grants, pinned: a change fails here.
PC_WANT_ARGV=$(printf '%s\n' -p --output-format text --allowedTools 'Read,Glob,Grep,Edit,Write,Bash')

printf '{"dispatch_id": "PLANT-D-1", "attempt_id": "A-1", "contract_digest": "x", "stage_id": "STAGE-002"}\n' > "$PC_TMP/d.json"
pc_adapter() {
    rm -f "$PC_TMP/fake.argv" "$PC_TMP/fake.out"
    PATH="$PC_PATH" FAKE_CLAUDE_MODE="$1" FAKE_CLAUDE_ARGV="$PC_TMP/fake.argv" FAKE_CLAUDE_OUT="$PC_TMP/fake.out" \
        "$PC_ADAPTER" < "$PC_TMP/d.json" > "$PC_TMP/adapter.out" 2> "$PC_TMP/adapter.err"
}
pc_adapter right
PC_STATUS=$?
if [[ $PC_STATUS -eq 0 ]] && [[ -s "$PC_TMP/adapter.out" ]] && cmp -s "$PC_TMP/fake.out" "$PC_TMP/adapter.out" \
    && [[ "$(cat "$PC_TMP/fake.argv")" == "$PC_WANT_ARGV" ]]; then
    pc_ok "the adapter passes the answer" "byte for byte; argv carries the pinned tool list"
else
    pc_fail "the adapter passes the answer" "exit $PC_STATUS; argv: $(tr '\n' ' ' < "$PC_TMP/fake.argv" 2> /dev/null); $(head -1 "$PC_TMP/adapter.err")"
fi
for PC_CASE in other:dispatch-mismatch resolve:self-completion notjson:not-json fail:claude-failed; do
    PC_MODE=${PC_CASE%%:*}
    PC_WHY=${PC_CASE#*:}
    pc_adapter "$PC_MODE"
    PC_STATUS=$?
    if [[ $PC_STATUS -ne 0 ]] && [[ ! -s "$PC_TMP/adapter.out" ]] && [[ -s "$PC_TMP/fake.out" ]] \
        && grep -q "refused: $PC_WHY:" "$PC_TMP/adapter.err"; then
        pc_ok "adapter refuses: $PC_MODE" "rejected by $PC_WHY (exit $PC_STATUS); stdout empty"
    else
        pc_fail "adapter refuses: $PC_MODE" "exit $PC_STATUS; stdout $(wc -c < "$PC_TMP/adapter.out") bytes; $(head -1 "$PC_TMP/adapter.err")"
    fi
done

# Through `war perform`: the good answer is ingested; each refusal records
# nothing.
pc_perform_adapter() {
    pc_reset
    sed -i "s|^performer_argv = .*|performer_argv = [\"$PC_ADAPTER\"]|" "$PC_ROOT/openwarrant.toml"
    assert_present "$PC_ADAPTER" "$PC_ROOT/openwarrant.toml"
    rm -f "$PC_TMP/fake.argv" "$PC_TMP/fake.out"
    PC_OUT=$(PATH="$PC_PATH" FAKE_CLAUDE_MODE="$1" FAKE_CLAUDE_ARGV="$PC_TMP/fake.argv" FAKE_CLAUDE_OUT="$PC_TMP/fake.out" \
        "$PC_WAR" --root "$PC_ROOT" perform "$PC_W" STAGE-002 --prototype 2>&1)
    PC_STATUS=$?
}
pc_perform_adapter right
if [[ $PC_STATUS -eq 0 ]] && grep -q 'perform.answered' <<< "$PC_OUT" && grep -q 'submission.recorded' <<< "$PC_OUT" \
    && [[ "$(pc_submissions)" == 1 ]] && [[ "$(cat "$PC_TMP/fake.argv")" == "$PC_WANT_ARGV" ]]; then
    pc_ok "war perform ingests the adapter" "perform.answered, submission.recorded"
else
    pc_fail "war perform ingests the adapter" "exit $PC_STATUS; submissions $(pc_submissions); $(grep -m1 -E 'ERROR|UNKNOWN' <<< "$PC_OUT")"
fi
for PC_CASE in other:dispatch-mismatch resolve:self-completion notjson:not-json fail:claude-failed; do
    PC_MODE=${PC_CASE%%:*}
    PC_WHY=${PC_CASE#*:}
    pc_perform_adapter "$PC_MODE"
    if [[ $PC_STATUS -ne 0 ]] && grep -q 'perform.failed' <<< "$PC_OUT" && grep -q "refused: $PC_WHY:" <<< "$PC_OUT" \
        && [[ -s "$PC_TMP/fake.out" ]] && [[ "$(pc_submissions)" == 0 ]] \
        && ! grep -q 'submission.recorded' <<< "$PC_OUT"; then
        pc_ok "war perform, adapter: $PC_MODE" "rejected by perform.failed ($PC_WHY); no submission"
    else
        pc_fail "war perform, adapter: $PC_MODE" "exit $PC_STATUS; submissions $(pc_submissions); $(grep -m1 -E 'ERROR|UNKNOWN|PASS' <<< "$PC_OUT")"
    fi
done

corpus_gone "$PC_ROOT"
rm -rf "$PC_TMP"
