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
# A generated projection is denied without consulting the pins.
plant_hook "edit under generated/ is denied" guard-pins.sh "$HKFX/edit-generated.json" 'war compile' present
# An unpinned file passes in silence.
plant_hook "edit to an unpinned file is silent" guard-pins.sh "$HKFX/edit-free.json" '' empty
# The stop check lets a stop through when it already blocked once this turn.
plant_hook "stop after a block is allowed" stop-check.sh "$HKFX/stop-active.json" '' empty

# The stop check blocks on a corpus `war check` refuses: plant a drift the
# checker sees, ask to stop, restore.
sed -i 's/^order: 10$/order: 11/' docs/warrants/OW-WAR-0062/atoms/10-intent.md
assert_present 'order: 11' docs/warrants/OW-WAR-0062/atoms/10-intent.md
plant_hook "stop on a red corpus is blocked" stop-check.sh "$HKFX/stop.json" '"decision":"block"' present
restore
