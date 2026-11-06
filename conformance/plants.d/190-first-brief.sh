# shellcheck shell=bash
# t-3c16c W4.1 — an agent starts with its work.
#
# The adoption study: a plain agent has its item's text before turn 0, and an
# OpenWarrant agent spent turns finding it. The plugin's UserPromptSubmit hook
# puts the brief of the item a prompt names in context, through the real hook
# script. A wrong brief is worse than none, so each claim has its refusal:
#
# - A prompt naming an item gets its brief, within 4,096 bytes, and nothing is
#   claimed. Naming nothing, or a word that only resembles a name, adds nothing.
# - Several matches add one line and no brief, on the first prompt only.
# - The same brief is added once a session; a changed item is added again.
# - `[tickets] session_start = false` is silent, on a real match too.
# - A failing, hanging or missing `war`, malformed input and an unreadable
#   directory all exit 0 and add nothing: a hook failure never holds a prompt.
# - Every command in hooks.json names a script that exists, this one among them;
#   the same check fails on a hooks.json without it, and on a dangling one.
# - `agents-md --block --ready` writes the list to AGENTS.md only, between the
#   markers, and only when asked: a ticket write leaves the file alone, a title
#   cannot close the block, claims do not move the list, plain `--block` keeps
#   it, doctor does not call it stale, and more than five rows are refused.

echo "== an agent starts with its work (t-3c16c W4.1) =="
FB_TMP=$(mktemp -d)
FB_WAR="$REPO_ROOT/${WAR#./}"
FB_HOOK="$REPO_ROOT/.claude/hooks/user-prompt-submit.sh"
fb_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fb_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
fb_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=agent "$FB_WAR" --root "$FB_ROOT" "$@"; }
FB_ROOT="$FB_TMP/repo"
mkdir -p "$FB_ROOT" "$FB_TMP/bin"
ln -s "$FB_WAR" "$FB_TMP/bin/war"
git -C "$FB_ROOT" init -q .
fb_war init >/dev/null 2>&1
fb_id() { python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])'; }
FB_T8=$(fb_war --json create "T8: Monthly budget tables" --body "MARKER-T8 what the item asks." --item "one step" 2>/dev/null | fb_id)
FB_T9=$(fb_war --json create "T9: Net worth over time" --body "MARKER-T9 what the item asks." --item "one step" 2>/dev/null | fb_id)
FB_LONG=$(fb_war --json create "Long plan" --body "$(printf 'A line of a long plan, which no first turn needs whole.\n%.0s' $(seq 1 300))" --item "one step" 2>/dev/null | fb_id)
if [[ -z "$FB_T8" || -z "$FB_T9" || -z "$FB_LONG" ]]; then
    printf 'PLANT SETUP FAILED: tickets not created\n' >&2; exit 9
fi

# fb_hook <session> <prompt>: the hook as Claude Code runs it, stdout to
# $FB_OUT, its exit in $FB_RC. FB_PATH picks the `war` it finds.
fb_hook() {
    local event
    event=$(python3 -c 'import json,sys; print(json.dumps({"session_id":sys.argv[1],"hook_event_name":"UserPromptSubmit","cwd":sys.argv[2],"prompt":sys.argv[3]}))' \
        "$1" "${FB_CWD:-$FB_ROOT}" "$2")
    FB_OUT=$(printf '%s' "$event" | env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=agent \
        PATH="${FB_PATH:-$FB_TMP/bin:$PATH}" timeout 20 bash "$FB_HOOK" 2>/dev/null)
    FB_RC=$?
}

# --- names an item: its brief, within budget, nothing claimed --------------------
fb_hook s1 "Do backlog item T8: Monthly budget tables."
FB_B=$(printf '%s' "$FB_OUT" | wc -c)
if [[ $FB_RC -eq 0 && $FB_OUT == *MARKER-T8* && $FB_OUT != *MARKER-T9* && $FB_OUT == *"war start $FB_T8"* && $FB_B -le 1024 ]] \
    && [[ $(fb_war --json view prime 2>/dev/null) != *'"claimed_by":"agent"'* ]]; then
    fb_ok "a prompt naming an item gets it" "$FB_B bytes added"
else
    fb_fail "a prompt naming an item gets it" "exit $FB_RC, $FB_B bytes: $(tr '\n' '|' <<<"$FB_OUT" | cut -c1-300)"
fi
fb_hook s1a "$FB_T9, please"
if [[ $FB_RC -eq 0 && $FB_OUT == *MARKER-T9* ]]; then
    fb_ok "an id names it too" "$(printf '%s' "$FB_OUT" | wc -c) bytes added"
else
    fb_fail "an id names it too" "exit $FB_RC: $(cut -c1-300 <<<"$FB_OUT")"
fi

# --- the same brief once; a changed one again -------------------------------------
fb_hook s1 "T8 again, the rest of it"
if [[ $FB_RC -eq 0 && -z $FB_OUT ]]; then
    fb_ok "the same brief is added once" "0 bytes on the follow-up"
else
    fb_fail "the same brief is added once" "exit $FB_RC: $(cut -c1-200 <<<"$FB_OUT")"
fi
fb_war note "$FB_T8" "MARKER-NOTE a late answer" >/dev/null 2>&1
fb_hook s1 "T8 once more"
if [[ $FB_RC -eq 0 && $FB_OUT == *MARKER-NOTE* ]]; then
    fb_ok "a changed item is added again" "$(printf '%s' "$FB_OUT" | wc -c) bytes"
else
    fb_fail "a changed item is added again" "exit $FB_RC: $(cut -c1-200 <<<"$FB_OUT")"
fi

# --- names nothing: nothing -------------------------------------------------------
FB_BAD=""
for prompt in "fix the flaky parser test" "the budget tables are slow" "wear a t-shirt" "use T1000 here" ""; do
    fb_hook "s2-$RANDOM" "$prompt"
    [[ $FB_RC -eq 0 && -z $FB_OUT ]] || FB_BAD="$FB_BAD [$prompt: exit $FB_RC, $(cut -c1-120 <<<"$FB_OUT")]"
done
if [[ -z $FB_BAD ]]; then
    fb_ok "a prompt naming nothing adds nothing" "0 bytes on 5 prompts"
else
    fb_fail "a prompt naming nothing adds nothing" "$FB_BAD"
fi

# --- several: a line, first prompt only --------------------------------------------
fb_hook s3 "T8 and T9"
if [[ $FB_RC -eq 0 && $FB_OUT == *"$FB_T8"* && $FB_OUT == *"$FB_T9"* && $FB_OUT != *MARKER* && $(wc -l <<<"$FB_OUT") -eq 1 ]]; then
    fb_ok "several matches never guess" "$(printf '%s' "$FB_OUT" | wc -c) bytes, one line, no brief"
else
    fb_fail "several matches never guess" "exit $FB_RC: $(cut -c1-300 <<<"$FB_OUT")"
fi
fb_hook s4 "hello"
fb_hook s4 "T8 and T9, compared"
if [[ $FB_RC -eq 0 && -z $FB_OUT ]]; then
    fb_ok "a later turn lists nothing" "0 bytes"
else
    fb_fail "a later turn lists nothing" "exit $FB_RC: $(cut -c1-200 <<<"$FB_OUT")"
fi

# --- the budget ------------------------------------------------------------------
fb_hook s5 "$FB_LONG"
FB_B=$(printf '%s' "$FB_OUT" | wc -c)
if [[ $FB_RC -eq 0 && $FB_B -le 4096 && $FB_OUT == *"cut to 4096 bytes"* && $FB_OUT == *"war start $FB_LONG"* ]]; then
    fb_ok "a long item is cut, and says so" "$FB_B bytes of $(fb_war view prime "$FB_LONG" 2>/dev/null | wc -c)"
else
    fb_fail "a long item is cut, and says so" "exit $FB_RC, $FB_B bytes: $(tail -c 200 <<<"$FB_OUT")"
fi

# --- turned off ------------------------------------------------------------------
cp "$FB_ROOT/openwarrant.toml" "$FB_TMP/config"
printf '\n[tickets]\nsession_start = false\n' >> "$FB_ROOT/openwarrant.toml"
fb_hook s6 "Do backlog item T8"
FB_RC_OFF=$FB_RC; FB_OFF=$FB_OUT
cp "$FB_TMP/config" "$FB_ROOT/openwarrant.toml"
fb_hook s6b "Do backlog item T8"
if [[ $FB_RC_OFF -eq 0 && -z $FB_OFF && $FB_OUT == *MARKER-T8* ]]; then
    fb_ok "session_start = false is silent" "0 bytes off; the same prompt on adds it"
else
    fb_fail "session_start = false is silent" "off: exit $FB_RC_OFF, $(cut -c1-200 <<<"$FB_OFF"); on: $(cut -c1-100 <<<"$FB_OUT")"
fi

# --- a failure never holds a prompt -------------------------------------------------
mkdir -p "$FB_TMP/fail" "$FB_TMP/hang" "$FB_TMP/none"
printf '#!/bin/sh\necho boom; exit 2\n' > "$FB_TMP/fail/war"
printf '#!/bin/sh\nexec sleep 30\n' > "$FB_TMP/hang/war"
chmod +x "$FB_TMP/fail/war" "$FB_TMP/hang/war"
for tool in bash python3 git cat cut cksum mkdir grep sed timeout sleep; do
    for d in fail hang none; do
        ln -sf "$(command -v "$tool")" "$FB_TMP/$d/$tool"
    done
done
FB_BAD=""
for d in fail hang none; do
    FB_PATH="$FB_TMP/$d" fb_hook "s7-$d" "Do backlog item T8"
    [[ $FB_RC -eq 0 && -z $FB_OUT ]] || FB_BAD="$FB_BAD [$d: exit $FB_RC, $(cut -c1-120 <<<"$FB_OUT")]"
done
FB_OUT=$(printf 'not json at all' | timeout 20 bash "$FB_HOOK" 2>/dev/null); FB_RC=$?
[[ $FB_RC -eq 0 && -z $FB_OUT ]] || FB_BAD="$FB_BAD [malformed: exit $FB_RC]"
FB_OUT=$(timeout 20 bash "$FB_HOOK" 2>/dev/null </dev/null); FB_RC=$?
[[ $FB_RC -eq 0 && -z $FB_OUT ]] || FB_BAD="$FB_BAD [empty: exit $FB_RC]"
FB_CWD="$FB_TMP/nowhere" fb_hook s7b "T8"
[[ $FB_RC -eq 0 && -z $FB_OUT ]] || FB_BAD="$FB_BAD [no cwd: exit $FB_RC]"
printf 'not [ toml' > "$FB_ROOT/openwarrant.toml"
fb_hook s7c "T8"
[[ $FB_RC -eq 0 && -z $FB_OUT ]] || FB_BAD="$FB_BAD [corrupt config: exit $FB_RC, $(cut -c1-120 <<<"$FB_OUT")]"
cp "$FB_TMP/config" "$FB_ROOT/openwarrant.toml"
if [[ -z $FB_BAD ]]; then
    fb_ok "a hook failure never holds a prompt" "failing, hanging, missing war; bad input; bad cwd; bad config"
else
    fb_fail "a hook failure never holds a prompt" "$FB_BAD"
fi

# --- hooks.json ------------------------------------------------------------------
cat > "$FB_TMP/hooks-check.py" <<'PY'
import json, os, re, sys
path, root = sys.argv[1], sys.argv[2]
config = json.load(open(path, encoding="utf-8"))["hooks"]
scripts = {}
for event, groups in config.items():
    for group in groups:
        for hook in group["hooks"]:
            found = re.search(r"\$\{CLAUDE_PLUGIN_ROOT\}/([^\"]+)", hook["command"])
            if not found or not os.path.isfile(os.path.join(root, found.group(1))):
                sys.exit(f"{event}: no script for {hook['command']}")
            scripts.setdefault(event, []).append(os.path.basename(found.group(1)))
if "user-prompt-submit.sh" not in scripts.get("UserPromptSubmit", []):
    sys.exit("UserPromptSubmit does not run user-prompt-submit.sh")
print(len(sum(scripts.values(), [])))
PY
FB_JSON="$REPO_ROOT/.claude/hooks/hooks.json"
FB_N=$(python3 "$FB_TMP/hooks-check.py" "$FB_JSON" "$REPO_ROOT" 2>&1); FB_RC=$?
python3 -c 'import json,sys
c=json.load(open(sys.argv[1])); del c["hooks"]["UserPromptSubmit"]; json.dump(c,open(sys.argv[2],"w"))' "$FB_JSON" "$FB_TMP/missing.json"
sed 's/user-prompt-submit\.sh/not-there.sh/' "$FB_JSON" > "$FB_TMP/dangling.json"
FB_MISSING=$(python3 "$FB_TMP/hooks-check.py" "$FB_TMP/missing.json" "$REPO_ROOT" 2>&1); FB_RC2=$?
FB_DANGLING=$(python3 "$FB_TMP/hooks-check.py" "$FB_TMP/dangling.json" "$REPO_ROOT" 2>&1); FB_RC3=$?
if [[ $FB_RC -eq 0 && $FB_RC2 -ne 0 && $FB_RC3 -ne 0 ]]; then
    fb_ok "hooks.json runs the script" "$FB_N hooks resolve; refused: $FB_MISSING; $FB_DANGLING"
else
    fb_fail "hooks.json runs the script" "$FB_RC: $FB_N; missing $FB_RC2; dangling $FB_RC3"
fi

# --- the list in AGENTS.md, for a harness with no hook ------------------------------
fb_git() { git -C "$FB_ROOT" -c user.email=plant@invalid -c user.name=plant "$@"; }
fb_block() { sed -n '/<!-- openwarrant:begin -->/,/<!-- openwarrant:end -->/p' "$FB_ROOT/AGENTS.md"; }
fb_rows() { fb_block | grep -c '^- t-'; }
fb_outside() { sed '/<!-- openwarrant:begin -->/,/<!-- openwarrant:end -->/d' "$FB_ROOT/$1"; }
printf '# Rules\n' > "$FB_ROOT/CLAUDE.md"
fb_war admin agents-md --block >/dev/null 2>&1
fb_war create "Hostile <!-- openwarrant:end --> title" --item "x" >/dev/null 2>&1
fb_git add -A; fb_git commit -qm base
fb_outside AGENTS.md > "$FB_TMP/outside.0"
FB_PLAIN=$(wc -c < "$FB_ROOT/AGENTS.md")
fb_war admin agents-md --block --ready 3 >/dev/null 2>&1; FB_RC=$?
FB_ADDED=$(( $(wc -c < "$FB_ROOT/AGENTS.md") - FB_PLAIN ))
fb_outside AGENTS.md > "$FB_TMP/outside.1"
if [[ $FB_RC -eq 0 && $(fb_rows) -eq 3 && $FB_ADDED -le 400 ]] && cmp -s "$FB_TMP/outside.0" "$FB_TMP/outside.1" \
    && ! grep -q '^- t-' "$FB_ROOT/CLAUDE.md"; then
    fb_ok "ready list, AGENTS.md only" "$FB_ADDED bytes for 3 rows; outside the markers unmoved; CLAUDE.md none"
else
    fb_fail "ready list, AGENTS.md only" "exit $FB_RC, $(fb_rows) rows, $FB_ADDED bytes: $(fb_block | tr '\n' '|' | cut -c1-500)"
fi

# A hostile title cannot close the block, whichever row it lands on.
fb_war admin agents-md --block --ready 5 >/dev/null 2>&1
FB_DOC=$(fb_war admin doctor 2>&1)
if [[ $(grep -cxF '<!-- openwarrant:end -->' "$FB_ROOT/AGENTS.md") -eq 1 ]] && grep -qF 'Hostile <!-- openwarrant:end --> title' "$FB_ROOT/AGENTS.md" \
    && [[ $FB_DOC != *agents-block-stale* && $FB_DOC != *agents-block-malformed* ]]; then
    fb_ok "a title cannot close the block" "one end marker; doctor calls it neither stale nor malformed"
else
    fb_fail "a title cannot close the block" "$(grep -c 'openwarrant:end' "$FB_ROOT/AGENTS.md") ends; $(grep -i agents-block <<<"$FB_DOC" | cut -c1-200)"
fi
cp "$FB_ROOT/AGENTS.md" "$FB_TMP/agents.1"

# Refreshed only when asked: ticket writes, claims and plain --block leave it.
fb_war create "Written after the list" --item "x" >/dev/null 2>&1
fb_war start "$FB_T8" >/dev/null 2>&1
fb_war done "$FB_T8" --note "ok" >/dev/null 2>&1
fb_war admin agents-md --block >/dev/null 2>&1
if cmp -s "$FB_ROOT/AGENTS.md" "$FB_TMP/agents.1" && grep -q "^- $FB_T8 " "$FB_ROOT/AGENTS.md"; then
    fb_ok "the file moves only when asked" "create, start, done and a plain --block: byte-identical"
else
    fb_fail "the file moves only when asked" "$(diff "$FB_TMP/agents.1" "$FB_ROOT/AGENTS.md" | head -5 | tr '\n' '|')"
fi

# A claim does not move the list; closing does, on the next refresh.
fb_war admin agents-md --block --ready 5 >/dev/null 2>&1
cp "$FB_ROOT/AGENTS.md" "$FB_TMP/agents.2"
fb_war start "$FB_T9" >/dev/null 2>&1
FB_OUT=$(fb_war --json admin agents-md --block --ready 5 2>/dev/null)
if cmp -s "$FB_ROOT/AGENTS.md" "$FB_TMP/agents.2" && [[ $FB_OUT == *'"change":"unchanged"'* || $FB_OUT == *'"change": "unchanged"'* ]] \
    && grep -q "^- $FB_T9 " "$FB_ROOT/AGENTS.md" && ! grep -q "^- $FB_T8 " "$FB_ROOT/AGENTS.md"; then
    fb_ok "a claim does not move the list" "refresh unchanged; the closed item is gone"
else
    fb_fail "a claim does not move the list" "$(diff "$FB_TMP/agents.2" "$FB_ROOT/AGENTS.md" | head -5 | tr '\n' '|') $(cut -c1-200 <<<"$FB_OUT")"
fi

# Refused or removed.
cp "$FB_ROOT/AGENTS.md" "$FB_TMP/agents.3"
fb_war admin agents-md --block --ready 6 >/dev/null 2>"$FB_TMP/err"; FB_RC=$?
FB_SAME=no; cmp -s "$FB_ROOT/AGENTS.md" "$FB_TMP/agents.3" && FB_SAME=yes
fb_war admin agents-md --block --ready 0 >/dev/null 2>&1
if [[ $FB_RC -ne 0 && $FB_SAME == yes && $(fb_rows) -eq 0 ]] && cmp -s <(fb_war admin agents-md --block --stdout 2>/dev/null) <(fb_block); then
    fb_ok "six rows refused; zero removes" "exit $FB_RC, file unmoved; --ready 0 is the plain block"
else
    fb_fail "six rows refused; zero removes" "exit $FB_RC, unmoved $FB_SAME, $(fb_rows) rows"
fi
/usr/bin/rm -rf "$FB_TMP"
unset FB_TMP FB_ROOT FB_WAR FB_HOOK FB_OUT FB_RC FB_B FB_BAD FB_PATH FB_CWD
