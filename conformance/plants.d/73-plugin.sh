# shellcheck shell=bash
# The Claude Code plugin's hooks (slice A6): a pin guard on edits and a stop
# check. Each plant feeds a committed hook event to the script and reads its
# decision, exactly as the harness would.

HKFX=conformance/fixtures/hooks
HOOKS=.claude/hooks

# Run a hook with the shipped binary first on PATH; print its stdout.
run_hook() {
    local script="$1" fixture="$2"
    PATH="$PWD/target/debug:$PATH" bash "$HOOKS/$script" < "$fixture" 2>/dev/null
}

plant_hook() {
    local name="$1" script="$2" fixture="$3" want="$4" mode="$5"
    local out
    out=$(run_hook "$script" "$fixture")
    local status=$?
    local ok=1
    case "$mode" in
        present) grep -Fq -- "$want" <<< "$out" && ok=0 ;;
        absent)  grep -Fq -- "$want" <<< "$out" || ok=0 ;;
        empty)   [[ -z "$out" ]] && ok=0 ;;
    esac
    if [[ $status -eq 0 && $ok -eq 0 ]]; then
        printf 'ok    %-34s %s %s\n' "$name" "$mode" "$want"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s exit %s; %s %s not satisfied\n      out: %s\n' "$name" "$status" "$mode" "$want" "$(cut -c1-300 <<< "$out")"
        FAILED=$((FAILED + 1))
    fi
}

# A file pinned by a resolved Warrant is denied, naming the Warrant and the act.
plant_hook "edit to a pinned file is denied" guard-pins.sh "$HKFX/edit-pinned.json" '"permissionDecision":"deny"' present
plant_hook "the denial names the correction act" guard-pins.sh "$HKFX/edit-pinned.json" 'war correct' present
# A pin a LATER authorized Warrant governs is historical (OW-ADR-0021): the
# hook lets the edit through in silence, because the newer Warrant answers
# for those bytes now. `check.rs` — OW-WAR-0005 resolved, OW-WAR-0112 owns.
plant_hook "edit to a historical pin is permitted" guard-pins.sh "$HKFX/edit-historical.json" '' empty
# A generated projection is denied without consulting the pins.
plant_hook "edit under generated/ is denied" guard-pins.sh "$HKFX/edit-generated.json" 'war compile' present
# M9: the generated/ rule is OpenWarrant's, so it holds only in a repository
# with an openwarrant.toml, and there only in the trees `war compile` writes
# (docs/ and the config's [paths]). In a repository without one, an edit to
# docs/generated/ passes in silence; once it adopts OpenWarrant the deny
# comes back, while its own codegen (src/generated/) stays ordinary work.
# The denial just above (this repository has one) pairs with the passes.
HK_FOREIGN=$(mktemp -d)
git -C "$HK_FOREIGN" init -q .
hk_event() {
    printf '{"session_id":"plant","hook_event_name":"PreToolUse","tool_name":"Write","cwd":"%s","tool_input":{"file_path":"%s/%s","content":"x"}}\n' \
        "$HK_FOREIGN" "$HK_FOREIGN" "$1" > "$HK_FOREIGN/$2.json"
}
hk_event docs/generated/CURRENT.md docs
hk_event src/generated/out.rs src
plant_hook "generated/ outside OpenWarrant passes" guard-pins.sh "$HK_FOREIGN/docs.json" '' empty
printf '[paths]\nwarrants = "plans/warrants"\n' > "$HK_FOREIGN/openwarrant.toml"
plant_hook "docs/generated/ once adopted is denied" guard-pins.sh "$HK_FOREIGN/docs.json" 'war compile' present
plant_hook "a repo's own codegen is not denied" guard-pins.sh "$HK_FOREIGN/src.json" '' empty
hk_event plans/warrants/X-WAR-0001/generated/WAR.md paths
plant_hook "a configured [paths] tree is guarded" guard-pins.sh "$HK_FOREIGN/paths.json" 'war compile' present
rm -rf "$HK_FOREIGN"
unset HK_FOREIGN
unset -f hk_event
# An unpinned file passes in silence.
plant_hook "edit to an unpinned file is silent" guard-pins.sh "$HKFX/edit-free.json" '' empty
# The stop check lets a stop through when it already blocked once this turn.
plant_hook "stop after a block is allowed" stop-check.sh "$HKFX/stop-active.json" '' empty

# The stop checks what the work changed (`war check --changed`). A tree in
# which nothing OpenWarrant governs moved stops in silence: no output, no
# context spent.
HK_MARK=$(git rev-parse --git-path openwarrant-stop)
rm -rf "$HK_MARK"
plant_hook "an unchanged tree stops in silence" stop-check.sh "$HKFX/stop.json" '' empty

# A change that breaks a Warrant holds the stop once, with the finding on
# stderr where the agent reads it. The same findings do not hold the next
# stop of the session, so a finding only a person can answer cannot loop.
sed -i 's/^order: 10$/order: 11/' docs/warrants/OW-WAR-0062/atoms/10-intent.md
assert_present 'order: 11' docs/warrants/OW-WAR-0062/atoms/10-intent.md
hk_said=$(PATH="$PWD/target/debug:$PATH" bash "$HOOKS/stop-check.sh" < "$HKFX/stop.json" 2>&1)
hk_status=$?
if [[ $hk_status -eq 2 && "$hk_said" == *"OpenWarrant: what changed does not check clean"* && "$hk_said" == *"ERROR"* ]]; then
    printf 'ok    a broken change holds the stop, naming it\n'
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  a broken change did not hold the stop: exit %s\n      out: %s\n' "$hk_status" "$(cut -c1-300 <<< "$hk_said")"
    FAILED=$((FAILED + 1))
fi
hk_again=$(PATH="$PWD/target/debug:$PATH" bash "$HOOKS/stop-check.sh" < "$HKFX/stop.json" 2>&1)
hk_status=$?
if [[ $hk_status -eq 0 && -z "$hk_again" ]]; then
    printf 'ok    the same findings hold one stop only\n'
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  the same findings held a second stop: exit %s\n      out: %s\n' "$hk_status" "$(cut -c1-300 <<< "$hk_again")"
    FAILED=$((FAILED + 1))
fi
restore
rm -rf "$HK_MARK"
unset HK_MARK hk_said hk_again hk_status

# Work the agent committed during the session is checked too: session-start
# records the commit the session began at, and the stop checks what changed
# since it, not only what is uncommitted. In a scratch repository, since a
# plant may not commit to this one.
HK_S=$(mktemp -d)
git -C "$HK_S" init -q .
(cd "$HK_S" && OPENWARRANT_NO_PROJECTS=1 "$REPO_ROOT/target/debug/war" init --program "Hook" --namespace HK >/dev/null 2>&1)
git -C "$HK_S" add -A
git -C "$HK_S" -c user.email=plant@example.invalid -c user.name=plant commit -qm scaffold
printf '{"session_id":"plant-s","hook_event_name":"SessionStart","cwd":"%s"}' "$HK_S" \
    | PATH="$REPO_ROOT/target/debug:$PATH" bash "$HOOKS/session-start.sh" >/dev/null 2>&1
sed -i 's/^order: 20$/order: 21/' "$HK_S/docs/warrants/HK-WAR-0001/atoms/20-basis.md"
git -C "$HK_S" -c user.email=plant@example.invalid -c user.name=plant commit -qam "a broken atom, committed"
hk_said=$(printf '{"session_id":"plant-s","hook_event_name":"Stop","stop_hook_active":false,"cwd":"%s"}' "$HK_S" \
    | PATH="$REPO_ROOT/target/debug:$PATH" bash "$HOOKS/stop-check.sh" 2>&1)
hk_status=$?
if [[ $hk_status -eq 2 && "$hk_said" == *"atom.header"* && "$hk_said" == *"HK-WAR-0001"* ]]; then
    printf 'ok    work committed in the session is checked\n'
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  work committed in the session was not checked: exit %s\n      out: %s\n' "$hk_status" "$(cut -c1-300 <<< "$hk_said")"
    FAILED=$((FAILED + 1))
fi
rm -rf "$HK_S"
unset HK_S hk_said hk_status
