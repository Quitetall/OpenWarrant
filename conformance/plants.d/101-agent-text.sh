# shellcheck shell=bash
# M9, decision 19: no agent-facing text reads as "you may not work".
#
# Every text an agent reads from this package is linted for an unscoped
# prohibition (conformance/agent-text/lint.py: "may not", "must not",
# "never", "forbidden", "not permitted", "stop", outside a section whose
# heading names a Warrant and its sign-off): the AGENTS.md template as
# `war agents-md` renders it, every Markdown file under .claude/skills/, the
# MCP server's instructions and tool descriptions as `war mcp` serves them,
# and the first line of each empty-state message in a repository `war init`
# just made. Each claim is paired with a refusal: the same lint, handed the
# same text with a prohibition planted in it, fails by file and line.

echo "== agent-facing text has no unscoped prohibition (M9) =="
AT_TMP=$(mktemp -d)
AT_LINT=conformance/agent-text/lint.py
at_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
at_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# What ships: the rendered template, the skills, and what `war mcp` says.
"$WAR" agents-md --stdout > "$AT_TMP/agents.md" 2>/dev/null
mapfile -t AT_SKILLS < <(LC_ALL=C find .claude/skills -name '*.md' | sort)
python3 conformance/fixtures/mcp/drive.py "$WAR" conformance/fixtures/mcp/list-tools.jsonl \
    > "$AT_TMP/mcp.jsonl" 2>/dev/null
python3 - "$AT_TMP/mcp.jsonl" "$AT_TMP/mcp-instructions.md" "$AT_TMP/mcp-tools.md" <<'PY'
import json, sys
instructions, tools = "", []
for line in open(sys.argv[1], encoding="utf-8"):
    r = json.loads(line).get("result", {})
    instructions = r.get("instructions", instructions)
    tools += [t["name"] + ": " + (t.get("description") or "") for t in r.get("tools", [])]
open(sys.argv[2], "w", encoding="utf-8").write(instructions + "\n")
open(sys.argv[3], "w", encoding="utf-8").write("\n".join(tools) + "\n")
PY
AT_ARGS=("template=$AT_TMP/agents.md" "mcp-instructions=$AT_TMP/mcp-instructions.md" \
    "mcp-tools=$AT_TMP/mcp-tools.md")
for f in "${AT_SKILLS[@]}"; do AT_ARGS+=("$f=$f"); done
AT_OUT=$(python3 "$AT_LINT" "${AT_ARGS[@]}" 2>&1)
AT_RC=$?
AT_TOOLS=$(wc -l < "$AT_TMP/mcp-tools.md")
if [[ $AT_RC -eq 0 && ${#AT_SKILLS[@]} -ge 8 && $AT_TOOLS -ge 20 ]] \
    && grep -q 'Ordinary work needs none of this' "$AT_TMP/mcp-instructions.md"; then
    at_ok "shipped agent text is clean" "template, ${#AT_SKILLS[@]} skill files, MCP instructions, $AT_TOOLS tool descriptions"
else
    at_fail "shipped agent text is clean" "exit $AT_RC, ${#AT_SKILLS[@]} skill files, $AT_TOOLS tools: $(head -3 <<<"$AT_OUT" | tr '\n' '|')"
fi

# The first line of each empty-state message, in a repository plain
# `war init` just made: nothing in it reads as "nothing to do" or a stop.
AT_REPO=$(mktemp -d)
git -C "$AT_REPO" init -q .
"$WAR" --root "$AT_REPO" init >/dev/null 2>&1
AT_EMPTY=()
for c in next ready tickets prime check; do
    "$WAR" --root "$AT_REPO" "$c" > "$AT_TMP/empty-$c.txt" 2>&1
    AT_EMPTY+=("war $c=$AT_TMP/empty-$c.txt")
done
AT_OUT=$(python3 "$AT_LINT" --first-line "${AT_EMPTY[@]}" 2>&1)
AT_RC=$?
AT_NEXT=$(head -1 "$AT_TMP/empty-next.txt")
AT_READY=$(head -1 "$AT_TMP/empty-ready.txt")
if [[ $AT_RC -eq 0 && $AT_NEXT == 'nothing tracked; work freely' \
    && $AT_READY == 'nothing tracked; work freely.'* ]]; then
    at_ok "empty states lead with work freely" "next, ready, tickets, prime, check"
else
    at_fail "empty states lead with work freely" "exit $AT_RC; next: $AT_NEXT; ready: $AT_READY; $(head -2 <<<"$AT_OUT" | tr '\n' '|')"
fi

# Refusals. A prohibition planted outside the scoped section is named by
# file and line; the same sentence inside the section that names a Warrant
# and its sign-off is allowed, which is what the scope is for.
awk 'NR==4{print "You may not edit code without a Warrant."} {print}' "$AT_TMP/agents.md" > "$AT_TMP/bad.md"
AT_OUT=$(python3 "$AT_LINT" "planted=$AT_TMP/bad.md" 2>&1)
AT_RC=$?
awk '{print} /^## When a Warrant.s type requires sign-off/{print ""; print "You may not edit code without a Warrant."}' \
    "$AT_TMP/agents.md" > "$AT_TMP/scoped.md"
AT_OUT2=$(python3 "$AT_LINT" "scoped=$AT_TMP/scoped.md" 2>&1)
AT_RC2=$?
if [[ $AT_RC -eq 1 ]] && grep -qF 'prohibition: planted:4: "may not"' <<<"$AT_OUT" && [[ $AT_RC2 -eq 0 ]]; then
    at_ok "a planted prohibition is refused" "planted:4 named; the same line in the scoped section passes"
else
    at_fail "a planted prohibition is refused" "unscoped exit $AT_RC ($(head -1 <<<"$AT_OUT")); scoped exit $AT_RC2"
fi
# In the MCP instructions, and in an empty state's first line.
printf 'Never edit a file without a Warrant.\n' > "$AT_TMP/bad-mcp.md"
printf 'nothing to do: stop here until a Warrant is signed\n' > "$AT_TMP/bad-empty.txt"
AT_OUT=$(python3 "$AT_LINT" "mcp-instructions=$AT_TMP/bad-mcp.md" 2>&1)
AT_RC=$?
AT_OUT2=$(python3 "$AT_LINT" --first-line "war next=$AT_TMP/bad-empty.txt" 2>&1)
AT_RC2=$?
if [[ $AT_RC -eq 1 && $AT_RC2 -eq 1 ]] && grep -qF '"Never"' <<<"$AT_OUT" \
    && grep -qF 'prohibition: war next:1: "stop"' <<<"$AT_OUT2"; then
    at_ok "planted MCP and empty-state refused" "\"Never\" in the instructions, \"stop\" in war next's first line"
else
    at_fail "planted MCP and empty-state refused" "exit $AT_RC/$AT_RC2: $AT_OUT | $AT_OUT2"
fi
# A text the lint could not read is UNKNOWN (exit 2), never clean.
AT_OUT=$(python3 "$AT_LINT" "gone=$AT_TMP/no-such-file.md" 2>&1)
AT_RC=$?
if [[ $AT_RC -eq 2 ]] && grep -q 'UNKNOWN' <<<"$AT_OUT"; then
    at_ok "an unreadable text is not clean" "exit 2, UNKNOWN"
else
    at_fail "an unreadable text is not clean" "exit $AT_RC: $AT_OUT"
fi

command rm -rf "$AT_TMP" "$AT_REPO"
unset AT_TMP AT_LINT AT_SKILLS AT_ARGS AT_OUT AT_OUT2 AT_RC AT_RC2 AT_TOOLS AT_REPO AT_EMPTY \
    AT_NEXT AT_READY
unset -f at_ok at_fail
