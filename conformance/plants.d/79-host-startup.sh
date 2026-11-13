# shellcheck shell=bash
# What `war` does before it knows its command (OW-WAR-0148 M8; SAS §82.2: a
# hosted run is a function of its request). 79-host.sh observes the whole of
# that purity and costs the pull-request tier more than its cap often leaves;
# this is the part every change to the entry point can break, in a second.
#
# Accepted: run from a scratch directory, `war host` (a model and a refusal)
# and `war --version` never ask for the working directory or name a path
# under it: nothing looks for a repository on behalf of a command that
# serves none. Refusal control: the same observation of `war status` there
# sees it ask. Without strace the answer is UNKNOWN, never passed.

echo "== war: nothing looks for a repository before the command is known =="
HS_TMP=$(mktemp -d)
HS_WAR="$REPO_ROOT/${WAR#./}"
HS_CASES="$REPO_ROOT/conformance/host/cases"
hs_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
hs_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# hs_asked <stdin-file> <war args...>: the traced lines that name the scratch
# working directory (a `getcwd` answers with it; a search from it names it).
# "untraced" where strace recorded no exec, so an empty answer is a real one.
hs_asked() {
    local req="$1" log="$HS_TMP/strace.log"
    shift
    mkdir -p "$HS_TMP/cwd"
    : > "$log"
    (cd "$HS_TMP/cwd" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_SESSION_ID \
        -u KATANA_SESSION_ID -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
        OPENWARRANT_NO_UPDATE_CHECK=1 strace -f -qq -o "$log" -e trace=%file \
        "$HS_WAR" "$@" < "$req" > /dev/null 2>&1) || true
    grep -q 'execve(' "$log" || { echo untraced; return; }
    grep -F "\"$HS_TMP/cwd" "$log" | head -2
}

if ! command -v strace >/dev/null 2>&1; then
    printf 'UNKNOWN %-32s strace is not installed; startup is not observed\n' "no repository sought at startup"
else
    hs_case() {
        local name="$1" bad
        shift
        bad=$(hs_asked "$@")
        if [[ -z "$bad" ]]; then
            hs_ok "no repository sought ($name)" "the working directory is never asked for"
        else
            hs_fail "no repository sought ($name)" "$bad"
        fi
    }
    hs_case "host" "$HS_CASES/warrant/request.json" host
    hs_case "host, refused" "$HS_CASES/refuse-version/request.json" host
    hs_case "--version" /dev/null --version
    # Refusal control: a command that does serve a repository is seen asking.
    hs_bad=$(hs_asked /dev/null status)
    if grep -q 'getcwd(' <<<"$hs_bad"; then
        hs_ok "the startup check sees a search" "war status asks for the working directory"
    else
        hs_fail "the startup check sees a search" "nothing seen: $hs_bad"
    fi
    unset -f hs_case
fi

command rm -rf "$HS_TMP"
unset HS_TMP HS_WAR HS_CASES hs_bad
unset -f hs_ok hs_fail hs_asked
