# shellcheck shell=bash
# OW-WAR-0148 M15 — `war start <id>`: one node, in a worktree of its own,
# with session-only settings, and the plugin guard inside it.
#
# One scratch program (GS), with HOME pointed at a directory of the plant's
# own that holds a `.claude/settings.json`. Warrant T declares, in
# atoms/35-allowed.md, the tool Read, the paths src/** and the command
# `make test`; Warrant E declares nothing; Warrant D is started with
# `[go] allowed_acts = "declared"`. Nothing runs a harness (`--print`).
#
# Accepted: `war start T/<item> --print` claims the item, makes a worktree on
# war-go/<node>, and writes `.claude/settings.local.json` there carrying the
# declared acts as permissions (Read, Edit(/src/**), Bash(make test)) and the
# claim's actor; the claim's journal line is in the worktree, not the
# checkout. In the worktree the guard lets an edit under src/ through.
# Refused: no file outside the worktree changes, in the checkout or in
# $HOME/.claude; the guard turns back an edit outside src/ in T's worktree,
# and says nothing about one in E's (nothing declared) or in the checkout;
# with `declared`, D's worktree gets no settings at all; a Warrant not yet
# committed is refused (go.start-uncommitted) and no worktree is made.

echo "== war start: a worktree with session-only settings (M15) =="
GS_ROOT=$(scratch_corpus GS)
[[ -d "${GS_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
GS_TMP=$(mktemp -d)
GS_WAR="$REPO_ROOT/${WAR#./}"
GS_GUARD="$REPO_ROOT/.claude/hooks/guard-pins.sh"
gs_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
gs_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
mkdir -p "$GS_TMP/home/.claude"
printf '{"permissions": {"allow": ["Bash(ls)"]}}\n' > "$GS_TMP/home/.claude/settings.json"
gsw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID HOME="$GS_TMP/home" "$GS_WAR" --root "$GS_ROOT" "$@" </dev/null; }
gs_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
gs_new() { gs_field "$(gsw --json create "$@" 2>/dev/null)" 'v["result"]["id"] + "/" + v["result"]["items"][0]["id"]'; }
# Every file outside .git and the worktrees, with its digest; and $HOME's.
gs_snapshot() {
    ( cd "$GS_ROOT" && find . -path ./.git -prune -o -path ./.openwarrant/worktrees -prune -o -type f -print0 \
        | sort -z | xargs -0 sha256sum )
    ( cd "$GS_TMP/home" && find . -type f -print0 | sort -z | xargs -0 sha256sum )
}
gs_guard() {
    printf '{"session_id":"plant","hook_event_name":"PreToolUse","tool_name":"Edit","cwd":"%s","tool_input":{"file_path":"%s/%s","old_string":"a","new_string":"b"}}\n' \
        "$1" "$1" "$2" | env -u SSH_AUTH_SOCK -u SSH_AGENT_PID PATH="/usr/bin:/bin" bash "$GS_GUARD" 2>/dev/null
}

GS_T=$(gs_new "Parser fix" -i "Fix the parser")
GS_E=$(gs_new "Anything goes" -i "Tidy up")
GS_D=$(gs_new "Declared only" -i "Look around")
printf '# Allowed\n\n## Tools\n- Read\n\n## Paths\n- src/**\n\n## Commands\n- `make test`\n' \
    > "$GS_ROOT/docs/tickets/${GS_T%/*}/atoms/35-allowed.md"
cp "$GS_ROOT/docs/tickets/${GS_T%/*}/atoms/35-allowed.md" "$GS_ROOT/docs/tickets/${GS_D%/*}/atoms/35-allowed.md"
git -C "$GS_ROOT" add -A >/dev/null 2>&1
git -C "$GS_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "three Warrants" >/dev/null 2>&1

GS_BEFORE=$(gs_snapshot)
GS_OUT=$(gsw --json start "$GS_T" --print 2>/dev/null); GS_RC=$?
GS_WT=$(gs_field "$GS_OUT" 'v["result"]["worktree"]')
GS_SETTINGS=$(python3 -c 'import json,sys; s=json.load(open(sys.argv[1])); print(s["permissions"]["allow"], s["env"]["OPENWARRANT_ACTOR"])' \
    "$GS_WT/.claude/settings.local.json" 2>/dev/null)
GS_BRANCH=$(git -C "$GS_WT" symbolic-ref --short HEAD 2>/dev/null)
GS_WT_CLAIM=$(grep -c '"type":"ticket.claimed"' "$GS_WT/docs/tickets/${GS_T%/*}/journal.jsonl" 2>/dev/null)
GS_COMMON=$(git -C "$GS_ROOT" rev-parse --path-format=absolute --git-common-dir)
if [[ $GS_RC -eq 0 && "$GS_SETTINGS" == "['Read', 'Edit(/src/**)', 'Bash(make test)'] claude" \
    && "$GS_BRANCH" == "war-go/${GS_T%/*}--${GS_T#*/}" && "$GS_WT_CLAIM" == "1" \
    && -f "$GS_COMMON/openwarrant/claims/${GS_T%/*}--${GS_T#*/}.lock" ]]; then
    gs_ok "war start makes a scoped session" "worktree on $GS_BRANCH, settings.local.json with the declared acts, claimed there"
else
    gs_fail "war start makes a scoped session" "exit $GS_RC; settings '$GS_SETTINGS'; branch '$GS_BRANCH'; claim lines $GS_WT_CLAIM"
fi
GS_AFTER=$(gs_snapshot)
GS_STATUS=$(git -C "$GS_ROOT" status --porcelain --untracked-files=all)
if [[ "$GS_BEFORE" == "$GS_AFTER" && -z "$GS_STATUS" ]]; then
    gs_ok "nothing written outside the worktree" "the checkout and \$HOME/.claude are byte for byte as they were"
else
    gs_fail "nothing written outside the worktree" "changed: $(diff <(echo "$GS_BEFORE") <(echo "$GS_AFTER") | head -3 | tr '\n' ' ') status: $GS_STATUS"
fi

# The guard: inside T's worktree, src/ passes and docs/ is turned back.
GS_IN=$(gs_guard "$GS_WT" src/parser.rs)
GS_OUTSIDE=$(gs_guard "$GS_WT" docs/notes.md)
if [[ -z "$GS_IN" && "$GS_OUTSIDE" == *'"permissionDecision":"deny"'* && "$GS_OUTSIDE" == *"src/**"* ]]; then
    gs_ok "the guard keeps to declared paths" "src/parser.rs passes; docs/notes.md is turned back, naming src/**"
else
    gs_fail "the guard keeps to declared paths" "src: '$GS_IN'; docs: '$GS_OUTSIDE'"
fi
GS_E_WT=$(gs_field "$(gsw --json start "$GS_E" --print 2>/dev/null)" 'v["result"]["worktree"]')
GS_E_EDIT=$(gs_guard "$GS_E_WT" docs/notes.md)
GS_ROOT_EDIT=$(gs_guard "$GS_ROOT" docs/notes.md)
if [[ -n "$GS_E_WT" && -z "$GS_E_EDIT" && -z "$GS_ROOT_EDIT" ]]; then
    gs_ok "no declared paths, no path rule" "an edit in E's worktree, and in the checkout, passes in silence"
else
    gs_fail "no declared paths, no path rule" "E: '$GS_E_EDIT'; checkout: '$GS_ROOT_EDIT'"
fi

# `declared`: shown, not written.
printf '\n[go]\nallowed_acts = "declared"\n' >> "$GS_ROOT/openwarrant.toml"
GS_D_OUT=$(gsw start "$GS_D" --print 2>/dev/null)
GS_D_WT="$GS_ROOT/.openwarrant/worktrees/${GS_D%/*}--${GS_D#*/}"
if [[ -d "$GS_D_WT" && ! -e "$GS_D_WT/.claude/settings.local.json" && ! -e "$GS_D_WT/.openwarrant/session.json" ]] \
    && grep -q '^declared paths: src/\*\*$' <<<"$GS_D_OUT"; then
    gs_ok "declared acts write no settings" "D's worktree has no settings and no session; its paths are shown"
else
    gs_fail "declared acts write no settings" "$(head -3 <<<"$GS_D_OUT" | tr '\n' ' ')"
fi

# Not committed: refused, and no worktree.
GS_U=$(gs_new "Not committed yet" -i "Later")
GS_U_OUT=$(gsw --json start "$GS_U" --print 2>/dev/null); GS_U_RC=$?
GS_U_RULE=$(gs_field "$GS_U_OUT" '",".join(d["rule"] for d in v["diagnostics"] if d["severity"] == "error")')
if [[ $GS_U_RC -eq 2 && "$GS_U_RULE" == "go.start-uncommitted" && ! -e "$GS_ROOT/.openwarrant/worktrees/${GS_U%/*}--${GS_U#*/}" ]]; then
    gs_ok "an uncommitted Warrant is refused" "go.start-uncommitted, and no worktree was made"
else
    gs_fail "an uncommitted Warrant is refused" "exit $GS_U_RC '$GS_U_RULE'"
fi

for w in "$GS_WT" "$GS_E_WT" "$GS_D_WT"; do git -C "$GS_ROOT" worktree remove --force "$w" >/dev/null 2>&1; done
command rm -rf "$GS_TMP"
unset GS_ROOT GS_TMP GS_WAR GS_GUARD GS_T GS_E GS_D GS_U GS_BEFORE GS_OUT GS_RC GS_WT GS_SETTINGS GS_BRANCH \
    GS_WT_CLAIM GS_COMMON GS_AFTER GS_STATUS GS_IN GS_OUTSIDE GS_E_WT GS_E_EDIT GS_ROOT_EDIT GS_D_OUT GS_D_WT \
    GS_U_OUT GS_U_RC GS_U_RULE
unset -f gs_ok gs_fail gsw gs_field gs_new gs_snapshot gs_guard
