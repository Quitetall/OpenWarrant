# shellcheck shell=bash
# M16 (decision 13): `war agents-md --block` keeps one small managed block,
# `<!-- openwarrant:begin -->` ... `<!-- openwarrant:end -->`, in the
# repository's CLAUDE.md and AGENTS.md. It points at `war prime` and says
# ordinary coding needs no Warrant; it never holds the active Warrant's
# context, and every byte outside its markers is the file owner's.
#
# Each claim is paired with an observed refusal:
# - Inserted into a CLAUDE.md that has none: the old file is a byte-for-byte
#   prefix of the new one, and one block follows it.
# - Run again: both files byte-identical, and the JSON says `unchanged`.
# - A stale block (an older stamp, with text after it) is rewritten and the
#   bytes before its begin marker and after its end marker do not move.
# - Refused, by name, with no file written: two blocks
#   (agents-md.block-duplicate) and a block that never closes
#   (agents-md.block-unterminated).
# - The block's text passes the agent-text lint; the same text with a
#   planted "stop" fails it.
# - `war init` adds the block to an existing CLAUDE.md and still writes the
#   full AGENTS.md; a CLAUDE.md whose block is malformed is left byte for
#   byte as it was, and init says so.

echo "== the managed pointer block (M16) =="
IB_TMP=$(mktemp -d)
IB_WAR="$REPO_ROOT/${WAR#./}"
ib_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ib_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ib_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$IB_WAR" --root "$IB_ROOT" "$@"; }
# Count of whole lines equal to $1 in file $2 (0 when none).
ib_count() { grep -cxF -- "$1" "$2" 2>/dev/null || true; }
IB_BEGIN='<!-- openwarrant:begin -->'
IB_END='<!-- openwarrant:end -->'

IB_ROOT="$IB_TMP/repo"
mkdir -p "$IB_ROOT"
git -C "$IB_ROOT" init -q .
ib_war init >/dev/null 2>&1
printf '# House rules\n\n## Testing\nRun `make test` before you push.\n\n## Style\nTabs, not spaces.' > "$IB_ROOT/CLAUDE.md"
cp "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.0"
cp "$IB_ROOT/AGENTS.md" "$IB_TMP/agents.0"

# --- inserted, outside bytes kept ---------------------------------------------
IB_OUT=$(ib_war agents-md --block 2>&1)
IB_RC=$?
IB_SIZE=$(wc -c < "$IB_TMP/claude.0")
if [[ $IB_RC -eq 0 ]] && cmp -s <(head -c "$IB_SIZE" "$IB_ROOT/CLAUDE.md") "$IB_TMP/claude.0" \
    && [[ $(ib_count "$IB_BEGIN" "$IB_ROOT/CLAUDE.md") == 1 && $(ib_count "$IB_END" "$IB_ROOT/CLAUDE.md") == 1 ]] \
    && grep -q 'war view prime' "$IB_ROOT/CLAUDE.md" && cmp -s "$IB_ROOT/AGENTS.md" "$IB_TMP/agents.0" \
    && grep -q '^CLAUDE.md: added the openwarrant block' <<<"$IB_OUT"; then
    ib_ok "block inserted, old bytes a prefix" "CLAUDE.md keeps its $IB_SIZE bytes; AGENTS.md (from init) already current"
else
    ib_fail "block inserted, old bytes a prefix" "exit $IB_RC: $(tr '\n' '|' <<<"$IB_OUT")"
fi
cp "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.1"

# --- idempotent -----------------------------------------------------------------
IB_JSON=$(ib_war agents-md --block --json 2>/dev/null)
IB_CHANGES=$(python3 -c 'import json,sys; v=json.loads(sys.argv[1]); print(" ".join(f["path"]+"="+f["change"] for f in v["result"]["files"]))' "$IB_JSON" 2>&1)
if cmp -s "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.1" && cmp -s "$IB_ROOT/AGENTS.md" "$IB_TMP/agents.0" \
    && [[ $IB_CHANGES == "AGENTS.md=unchanged CLAUDE.md=unchanged" ]]; then
    ib_ok "a second run changes nothing" "$IB_CHANGES; both files byte-identical"
else
    ib_fail "a second run changes nothing" "$IB_CHANGES"
fi

# --- a stale block is rewritten in place -------------------------------------
sed -i 's/written by war [^ ]* -->/written by war 0.0.1 -->/' "$IB_ROOT/CLAUDE.md"
printf '\n## Added after the block\nStill mine.\n' >> "$IB_ROOT/CLAUDE.md"
cp "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.2"
IB_OUT=$(ib_war agents-md --block 2>&1)
IB_RC=$?
IB_OUTSIDE=$(python3 - "$IB_TMP/claude.2" "$IB_ROOT/CLAUDE.md" "$IB_BEGIN" "$IB_END" <<'PY' 2>&1
import sys
a, b = open(sys.argv[1], "rb").read(), open(sys.argv[2], "rb").read()
begin, end = sys.argv[3].encode(), sys.argv[4].encode()
def parts(x):
    i = x.index(begin); j = x.index(end) + len(end)
    return x[:i], x[i:j], x[j:]
(pa, ba, ta), (pb, bb, tb) = parts(a), parts(b)
print("before", pa == pb, "after", ta == tb, "block", ba != bb, "stamp", b"war 0.0.1" not in bb)
PY
)
if [[ $IB_RC -eq 0 && $IB_OUTSIDE == "before True after True block True stamp True" ]] \
    && grep -q '^CLAUDE.md: updated the openwarrant block' <<<"$IB_OUT"; then
    ib_ok "a stale block is rewritten in place" "bytes before the begin and after the end marker unchanged"
else
    ib_fail "a stale block is rewritten in place" "exit $IB_RC: $IB_OUTSIDE | $(tr '\n' '|' <<<"$IB_OUT")"
fi
cp "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.3"

# --- refusals: two blocks, an unterminated block --------------------------------
cat "$IB_TMP/claude.3" "$IB_TMP/claude.3" > "$IB_ROOT/CLAUDE.md"
cp "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.dup"
sed -i 's/written by war [^ ]* -->/written by war 0.0.1 -->/' "$IB_ROOT/AGENTS.md"
cp "$IB_ROOT/AGENTS.md" "$IB_TMP/agents.old"
IB_OUT=$(ib_war agents-md --block 2>&1)
IB_RC=$?
if [[ $IB_RC -ne 0 ]] && grep -q 'agents-md.block-duplicate' <<<"$IB_OUT" \
    && cmp -s "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.dup" && cmp -s "$IB_ROOT/AGENTS.md" "$IB_TMP/agents.old"; then
    ib_ok "two blocks are refused by name" "agents-md.block-duplicate; neither file written (AGENTS.md stays stale)"
else
    ib_fail "two blocks are refused by name" "exit $IB_RC: $(tr '\n' '|' <<<"$IB_OUT")"
fi
printf '# Mine\n\n%s\nhalf a block\n' "$IB_BEGIN" > "$IB_ROOT/CLAUDE.md"
cp "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.open"
IB_OUT=$(ib_war agents-md --block 2>&1)
IB_RC=$?
if [[ $IB_RC -ne 0 ]] && grep -q 'agents-md.block-unterminated' <<<"$IB_OUT" && grep -q 'CLAUDE.md:3' <<<"$IB_OUT" \
    && cmp -s "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.open" && cmp -s "$IB_ROOT/AGENTS.md" "$IB_TMP/agents.old"; then
    ib_ok "an unterminated block is refused" "agents-md.block-unterminated at CLAUDE.md:3; nothing written"
else
    ib_fail "an unterminated block is refused" "exit $IB_RC: $(tr '\n' '|' <<<"$IB_OUT")"
fi

# --- the block's own words pass the agent-text lint ---------------------------
"$IB_WAR" agents-md --block --stdout > "$IB_TMP/block.md" 2>/dev/null
sed 's/^Ordinary coding/Stop. Ordinary coding/' "$IB_TMP/block.md" > "$IB_TMP/block-bad.md"
IB_LINT=$(python3 conformance/agent-text/lint.py "block=$IB_TMP/block.md" 2>&1)
IB_LINT_RC=$?
IB_LINT2=$(python3 conformance/agent-text/lint.py "planted=$IB_TMP/block-bad.md" 2>&1)
IB_LINT2_RC=$?
if [[ $IB_LINT_RC -eq 0 && $IB_LINT2_RC -eq 1 ]] && grep -q 'Ordinary coding needs no Warrant' "$IB_TMP/block.md" \
    && grep -qF '"Stop"' <<<"$IB_LINT2"; then
    ib_ok "the block's text passes the lint" "and a planted \"Stop\" in it is refused"
else
    ib_fail "the block's text passes the lint" "clean exit $IB_LINT_RC ($IB_LINT); planted exit $IB_LINT2_RC ($IB_LINT2)"
fi

# --- war init: an existing CLAUDE.md gets the block --------------------------------
IB_ROOT="$IB_TMP/init"
mkdir -p "$IB_ROOT"
git -C "$IB_ROOT" init -q .
cp "$IB_TMP/claude.0" "$IB_ROOT/CLAUDE.md"
IB_OUT=$(ib_war init 2>&1)
IB_RC=$?
if [[ $IB_RC -eq 0 ]] && cmp -s <(head -c "$IB_SIZE" "$IB_ROOT/CLAUDE.md") "$IB_TMP/claude.0" \
    && [[ $(ib_count "$IB_BEGIN" "$IB_ROOT/CLAUDE.md") == 1 ]] \
    && grep -q '^## Tracking work with tickets (optional)' "$IB_ROOT/AGENTS.md" \
    && [[ $(ib_count "$IB_END" "$IB_ROOT/AGENTS.md") == 1 ]] \
    && grep -q '^CLAUDE.md: added the openwarrant block' <<<"$IB_OUT"; then
    ib_ok "init adds the block to CLAUDE.md" "its bytes kept; the full AGENTS.md written, ending in the block"
else
    ib_fail "init adds the block to CLAUDE.md" "exit $IB_RC: $(tr '\n' '|' <<<"$IB_OUT")"
fi
IB_ROOT="$IB_TMP/init-bad"
mkdir -p "$IB_ROOT"
git -C "$IB_ROOT" init -q .
cp "$IB_TMP/claude.dup" "$IB_ROOT/CLAUDE.md"
IB_OUT=$(ib_war init 2>&1)
IB_RC=$?
if [[ $IB_RC -eq 0 ]] && cmp -s "$IB_ROOT/CLAUDE.md" "$IB_TMP/claude.dup" \
    && grep -q '^left as it is: CLAUDE.md: .*(agents-md.block-duplicate)' <<<"$IB_OUT" \
    && [[ -f "$IB_ROOT/openwarrant.toml" ]]; then
    ib_ok "init leaves a malformed block alone" "CLAUDE.md byte-identical; init finished and says why"
else
    ib_fail "init leaves a malformed block alone" "exit $IB_RC: $(tr '\n' '|' <<<"$IB_OUT")"
fi

command rm -rf "$IB_TMP"
unset IB_TMP IB_WAR IB_ROOT IB_OUT IB_RC IB_SIZE IB_JSON IB_CHANGES IB_OUTSIDE IB_LINT IB_LINT2 \
    IB_LINT_RC IB_LINT2_RC IB_BEGIN IB_END
unset -f ib_ok ib_fail ib_war ib_count
