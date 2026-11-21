# shellcheck shell=bash
# Build slots: `war run <slot> -- CMD…` (OW-ADR-0037).
#
# Each claim beside its refusal:
# - A second `war run build` waits, names the holder (actor and pid) once,
#   then runs with the resources env, after the first has ended. Refused: a
#   live holder keeps it waiting, and its command does not run.
# - The exit code is CMD's (7 is 7, 0 is 0).
# - A holder whose process is gone is taken over, and that is said.
# - A holder that cannot be read is UNKNOWN: exit 2, CMD not run.

echo "== resources: build slots (OW-ADR-0037) =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID OPENCODE_SESSION_ID OPENCODE
RS_WAR="$REPO_ROOT/${WAR#./}"
rs_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rs_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
RS_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$RS_T")
RS_R="$RS_T/repo" && mkdir -p "$RS_R"
RS_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner
    OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
rs_war() { "${RS_ENV[@]}" "$RS_WAR" --root "$RS_R" "$@" </dev/null; }
(cd "$RS_R" && "${RS_ENV[@]}" git init -q -b main && "${RS_ENV[@]}" "$RS_WAR" init >/dev/null 2>&1) \
    || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
cat >> "$RS_R/openwarrant.toml" <<'EOF'

[resources.env]
RS_BUILD_DIR = "{common}/openwarrant/build"
[resources.slots]
build = 1
EOF
RS_SLOT="$(realpath "$RS_R/.git")/openwarrant/slots/build.0.json"
rs_wait_slot() { local i; for i in $(seq 1 50); do [[ -s "$RS_SLOT" ]] && return 0; sleep 0.1; done; return 1; }

# A holder, then a second that waits on it.
"${RS_ENV[@]}" OPENWARRANT_ACTOR=first "$RS_WAR" --root "$RS_R" run build -- sh -c "sleep 2; echo first-done > '$RS_T/first'" </dev/null >/dev/null 2>&1 &
RS_HOLDER=$!
rs_wait_slot || { printf 'PLANT SETUP FAILED: the first holder never took the slot\n' >&2; exit 9; }
out=$(rs_war run build -- sh -c "test -s '$RS_T/first' && echo \"second \$RS_BUILD_DIR\"" 2>&1); rc=$?
wait "$RS_HOLDER" 2>/dev/null
if [[ $rc -eq 0 && "$out" == *"waiting on first (pid $RS_HOLDER,"* && "$(grep -c 'waiting on' <<<"$out")" -eq 1 \
    && "$out" == *"second $(realpath "$RS_R/.git")/openwarrant/build"* && ! -e "$RS_SLOT" ]]; then
    rs_ok "a second run waits, then runs" "named first (pid $RS_HOLDER) once; ran after it with the env"
else
    rs_fail "a second run waits, then runs" "rc $rc: $out"
fi

rs_war run build -- sh -c 'exit 7' >/dev/null 2>&1; rc7=$?
rs_war run build -- true >/dev/null 2>&1; rc0=$?
[[ $rc7 -eq 7 && $rc0 -eq 0 ]] && rs_ok "the exit code is the command's" "7 and 0" \
    || rs_fail "the exit code is the command's" "got $rc7 and $rc0"

# A dead holder is taken over; a live one is not.
sh -c 'exit 0' & RS_DEAD=$!; wait "$RS_DEAD"
mkdir -p "$(dirname "$RS_SLOT")"
printf '{"schema":"oh.war/slot/v1","slot":"build","actor":"ghost","pid":%s,"since":1,"token":"x"}' "$RS_DEAD" > "$RS_SLOT"
out=$(rs_war run build -- echo ran-after-takeover 2>&1); rc=$?
if [[ $rc -eq 0 && "$out" == *"holder ghost (pid $RS_DEAD) is gone; taken over"* && "$out" == *ran-after-takeover* ]]; then
    rs_ok "a dead holder is taken over" "said, and the command ran"
else
    rs_fail "a dead holder is taken over" "rc $rc: $out"
fi
sleep 30 & RS_LIVE=$!
printf '{"schema":"oh.war/slot/v1","slot":"build","actor":"alive","pid":%s,"since":1,"token":"y"}' "$RS_LIVE" > "$RS_SLOT"
out=$("${RS_ENV[@]}" timeout 2 "$RS_WAR" --root "$RS_R" run build -- echo SHOULD-NOT-RUN </dev/null 2>&1); rc=$?
kill "$RS_LIVE" 2>/dev/null; wait "$RS_LIVE" 2>/dev/null
if [[ $rc -eq 124 && "$out" == *"waiting on alive (pid $RS_LIVE,"* && "$out" != *SHOULD-NOT-RUN* ]]; then
    rs_ok "a live holder is waited on" "alive (pid $RS_LIVE) named; nothing ran"
else
    rs_fail "a live holder is waited on" "rc $rc: $out"
fi

echo 'not a holder' > "$RS_SLOT"
out=$(rs_war run build -- echo SHOULD-NOT-RUN 2>&1); rc=$?
if [[ $rc -eq 2 && "$out" == *"UNKNOWN (resources.slot-unknown)"* && "$out" != *SHOULD-NOT-RUN* && -s "$RS_SLOT" ]]; then
    rs_ok "an unreadable slot is UNKNOWN" "exit 2; nothing ran; the file kept"
else
    rs_fail "an unreadable slot is UNKNOWN" "rc $rc: $out"
fi
