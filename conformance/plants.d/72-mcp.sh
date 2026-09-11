# shellcheck shell=bash
# `war mcp` — the Model Context Protocol server holds no authority (OW-ADR-0014).
# Each plant feeds a committed JSON-RPC transcript to the server (drive.py waits
# for each answer before the next request, so EOF never races an in-flight
# call) and reads what it answered; a server that could sign would show it here.

MCPFX=conformance/fixtures/mcp

# Feed a transcript; pass when the expected text is (or is not) in the reply.
plant_mcp() {
    local name="$1" want="$2" mode="$3" fixture="$4"
    local out
    out=$(python3 "$MCPFX/drive.py" "$WAR" "$fixture" 2>/dev/null)
    local status=$?
    local ok=1
    # A here-string, not a pipe: under lib.sh's `pipefail`, `grep -q` closing
    # the pipe on its first match hands printf a SIGPIPE and the pipeline
    # reports failure for a reply that DID match — the longer the reply, the
    # more reliably. This plant failed only inside the full battery for that.
    case "$mode" in
        present) grep -Fq -- "$want" <<< "$out" && ok=0 ;;
        absent)  grep -Fq -- "$want" <<< "$out" || ok=0 ;;
    esac
    if [[ $status -eq 0 && $ok -eq 0 ]]; then
        printf 'ok    %-34s %s %s\n' "$name" "$mode" "$want"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s exit %s; %s %s not satisfied\n' "$name" "$status" "$mode" "$want"
        printf '      reply: %s\n' "$(printf '%s' "$out" | cut -c1-400 | head -8)"
        FAILED=$((FAILED + 1))
    fi
}

# tools/list carries no signing act under any spelling (only the read-only list/show).
plant_mcp "tools/list has no signing tool" '"name":"war_sign"' absent "$MCPFX/list-tools.jsonl"
plant_mcp "tools/list has no ingest tool" '_ingest"' absent "$MCPFX/list-tools.jsonl"
plant_mcp "tools/list does list war_next" '"name":"war_next"' present "$MCPFX/list-tools.jsonl"

# Calling a signing or ingesting tool is refused by the protocol, not by a message.
plant_mcp "calling war_sign is refused" 'tool not found' present "$MCPFX/call-sign.jsonl"

# The request halves write nothing: the tree is byte-identical afterwards.
BEFORE=$(git status --porcelain)
plant_mcp "request halves answer" 'authorize.request' present "$MCPFX/request-halves.jsonl"
AFTER=$(git status --porcelain)
if [[ "$BEFORE" == "$AFTER" ]]; then
    printf 'ok    %-34s tree unchanged after five request-half calls\n' "request halves write nothing"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s tree changed:\n%s\n' "request halves write nothing" "$(diff <(printf '%s' "$BEFORE") <(printf '%s' "$AFTER"))"
    FAILED=$((FAILED + 1))
fi
# …and an unreviewed apply is refused in the same transcript.
plant_mcp "unreviewed plan_apply is refused" 'step 6' present "$MCPFX/request-halves.jsonl"

# Outside a repository the server never starts: an ordinary CLI refusal, exit 1.
if (cd /tmp && "$OLDPWD/$WAR" mcp < /dev/null >/dev/null 2>&1); then
    printf 'FAIL  %-34s served outside a repository\n' "mcp outside a repo exits 1"
    FAILED=$((FAILED + 1))
else
    printf 'ok    %-34s refused before any runtime existed\n' "mcp outside a repo exits 1"
    PASSED=$((PASSED + 1))
fi
