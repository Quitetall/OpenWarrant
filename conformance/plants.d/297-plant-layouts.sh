# shellcheck shell=bash
# The battery runs in a named ticket layout, and once per layout
# (conformance/lib.sh `plant_layout`, conformance/plant-layouts.sh). Each claim
# is paired with its refusal.
#
# Accepted: under OW_PLANT_LAYOUT=work `war create` in a plant writes
# docs/work/, under tickets docs/tickets/, whatever the caller exported; a
# file that pins a layout runs in it, and the file after it does not;
# `plant-layouts.sh` runs each layout in its own clone of HEAD and passes when
# both do; `--pins` lists a pin with its reason.
# Refused: a layout that is neither (exit 1, nothing run); a pin without a
# reason (PLANT SETUP FAILED, exit 9); one layout failing fails the run and is
# named, the other still reported; a run that died is a failure, not a pass; an
# unknown plant name and a nested run (exit 2).

echo "== the battery per ticket layout =="
LY_TMP=$(mktemp -d)
ly_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ly_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
LY_SESSION=(-u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID -u CODEX_THREAD_ID -u CODEX_SESSION_ID
    -u CLAUDE_CODE_SESSION_ID)

# Three plant files of this plant's own: one creates a ticket and says where it
# landed, one pins `tickets` first, one pins with no reason.
mkdir -p "$LY_TMP/p"
cat >"$LY_TMP/p/10-where.sh" <<'FIXTURE'
LY_R=$(scratch_corpus LY)
(cd "$LY_R" && OPENWARRANT_ACTOR=plant "$REPO_ROOT/${WAR#./}" create "Where does this land" --item one >/dev/null 2>&1)
printf 'WHERE %s %s tickets=%s work=%s\n' "${LY_NAME:-plain}" "${OPENWARRANT_TICKET_LAYOUT:-unset}" \
    "$(find "$LY_R/docs/tickets" -name 't-*.md' 2>/dev/null | wc -l)" \
    "$(find "$LY_R/docs/work" -name 'w-*.md' 2>/dev/null | wc -l)"
corpus_gone "$LY_R"
FIXTURE
{
    printf 'plant_layout tickets "this fixture is about the older layout"\nLY_NAME=pinned\n'
    cat "$LY_TMP/p/10-where.sh"
    printf 'unset LY_NAME\n'
} >"$LY_TMP/p/05-pinned.sh"
printf 'plant_layout tickets\necho REACHED\n' >"$LY_TMP/p/20-no-reason.sh"

# ly_run <OW_PLANT_LAYOUT or ''> <caller's OPENWARRANT_TICKET_LAYOUT or ''> <file...>
ly_run() {
    local layout=$1 caller=$2
    shift 2
    (cd "$REPO_ROOT" && env "${LY_SESSION[@]}" -u OW_PLANT_LAYOUT -u OPENWARRANT_TICKET_LAYOUT \
        ${layout:+OW_PLANT_LAYOUT="$layout"} ${caller:+OPENWARRANT_TICKET_LAYOUT="$caller"} \
        bash -c 'source conformance/lib.sh >/dev/null; run_plant_files "$@"; echo "TOTALS $PASSED $FAILED"' _ "$@" 2>&1)
}

LY_W=$(ly_run work tickets "$LY_TMP/p/10-where.sh" | grep '^WHERE')
LY_K=$(ly_run tickets work "$LY_TMP/p/10-where.sh" | grep '^WHERE')
if [[ "$LY_W" == "WHERE plain work tickets=0 work=1" && "$LY_K" == "WHERE plain tickets tickets=1 work=0" ]]; then
    ly_ok "the battery's layout, not the caller's" "work writes docs/work/, tickets docs/tickets/, each with the other exported"
else
    ly_fail "the battery's layout, not the caller's" "work: '$LY_W' tickets: '$LY_K'"
fi

LY_P=$(ly_run work '' "$LY_TMP/p/05-pinned.sh" "$LY_TMP/p/10-where.sh" | grep '^WHERE' | tr '\n' '|')
if [[ "$LY_P" == "WHERE pinned tickets tickets=1 work=0|WHERE plain work tickets=0 work=1|" ]]; then
    ly_ok "a pin holds for its file only" "pinned tickets under a work battery; the next file is in work again"
else
    ly_fail "a pin holds for its file only" "$LY_P"
fi

LY_BAD=$(ly_run masters '' "$LY_TMP/p/10-where.sh"); LY_BAD_RC=$?
LY_NR=$(ly_run work '' "$LY_TMP/p/20-no-reason.sh"); LY_NR_RC=$?
if [[ $LY_BAD_RC -eq 1 && "$LY_BAD" == *"OW_PLANT_LAYOUT must be tickets or work, not 'masters'"* && "$LY_BAD" != *WHERE* \
    && $LY_NR_RC -eq 9 && "$LY_NR" == *"PLANT SETUP FAILED: plant_layout <tickets|work> <reason>"* && "$LY_NR" != *REACHED* ]]; then
    ly_ok "no such layout, no reason" "an unknown layout runs nothing (exit 1); a pin without a reason stops the battery (exit 9)"
else
    ly_fail "no such layout, no reason" "layout rc $LY_BAD_RC: $(head -c 160 <<<"$LY_BAD") | pin rc $LY_NR_RC: $(head -c 160 <<<"$LY_NR")"
fi

# --- plant-layouts.sh, over a fake battery in a scratch repository ---------------------
# The real battery inside the battery would recurse (48-isolated-battery.sh).
LY_REPO="$LY_TMP/repo"
mkdir -p "$LY_REPO/conformance/plants.d" "$LY_REPO/target/debug"
command cp conformance/plant-layouts.sh "$LY_REPO/conformance/"
printf '#!/bin/sh\nexit 0\n' >"$LY_REPO/target/debug/war"; chmod +x "$LY_REPO/target/debug/war"
printf 'import sys\nsys.exit(0)\n' >"$LY_REPO/conformance/views.py"
printf 'plant_layout tickets "about the older layout"\n' >"$LY_REPO/conformance/plants.d/10-old.sh"
printf 'echo no pin here\n' >"$LY_REPO/conformance/plants.d/20-any.sh"
cat >"$LY_REPO/conformance/plant.sh" <<'FAKE'
#!/usr/bin/env bash
# $1 $2 are --layout <layout>.
echo "fake battery in $2 sees $(cat conformance/probe.txt), files: $(tr '\n' ' ' <"${OW_PLANT_FILES:-/dev/null}")"
if [[ "$2" == "${LY_DIE:-}" ]]; then kill -9 $$; fi
if [[ "$2" == "${LY_FAILS:-}" ]]; then echo "FAIL  planted                            in $2"; echo "0 passed, 1 failed"; exit 1; fi
echo "1 passed, 0 failed"
FAKE
printf 'committed\n' >"$LY_REPO/conformance/probe.txt"
printf 'target\n' >"$LY_REPO/.gitignore"
(cd "$LY_REPO" && git init -q . && git add -A && git -c user.email=plant@invalid -c user.name=plant commit -qm fake) >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: no fake repository for plant-layouts.sh\n' >&2; exit 9; }
printf 'uncommitted\n' >"$LY_REPO/conformance/probe.txt"
# ly_wrap <env...> -- <args...>: the wrapper as the outermost run.
ly_wrap() {
    local -a envs=()
    while [[ "$1" != -- ]]; do envs+=("$1"); shift; done
    shift
    (cd "$LY_REPO" && env "${LY_SESSION[@]}" -u OPENWARRANT_IN_BATTERY TMPDIR="$LY_TMP" OW_PLANT_LAYOUT_LOGS="$LY_TMP/logs" \
        ${envs[@]+"${envs[@]}"} bash conformance/plant-layouts.sh "$@" 2>&1)
}

LY_BOTH=$(ly_wrap -- 20-any.sh); LY_BOTH_RC=$?
LY_SEEN=$(cat "$LY_TMP/logs/tickets.log" "$LY_TMP/logs/work.log" 2>/dev/null | grep -c 'sees committed, files: 20-any.sh')
if [[ $LY_BOTH_RC -eq 0 && "$LY_BOTH" == *"layout tickets: PASS"*"layout work: PASS"* && $LY_SEEN -eq 2 \
    && "$(cat "$LY_REPO/conformance/probe.txt")" == uncommitted ]]; then
    ly_ok "both layouts, each in a clone" "two runs of HEAD's bytes with the named plant; the working tree untouched"
else
    ly_fail "both layouts, each in a clone" "exit $LY_BOTH_RC seen $LY_SEEN: $(tr '\n' '|' <<<"$LY_BOTH" | head -c 300)"
fi

LY_ONE=$(ly_wrap LY_FAILS=work --); LY_ONE_RC=$?
LY_DEAD=$(ly_wrap LY_DIE=tickets --); LY_DEAD_RC=$?
if [[ $LY_ONE_RC -eq 1 && "$LY_ONE" == *"layout tickets: PASS"*"FAIL  planted"*"layout work: FAIL (plant.sh exit 1)"* \
    && $LY_DEAD_RC -eq 1 && "$LY_DEAD" == *"layout tickets: FAIL (plant.sh exit 137)"*"layout work: PASS"* ]]; then
    ly_ok "one layout failing fails the run" "the failing layout is named beside the passing one; a run that died is no pass"
else
    ly_fail "one layout failing fails the run" "fail rc $LY_ONE_RC: $(tr '\n' '|' <<<"$LY_ONE" | head -c 240) | dead rc $LY_DEAD_RC: $(tr '\n' '|' <<<"$LY_DEAD" | head -c 240)"
fi

LY_PINS=$(ly_wrap -- --pins)
LY_UNK=$(ly_wrap -- 99-nothing.sh); LY_UNK_RC=$?
LY_LAY=$(ly_wrap -- --layouts work,masters); LY_LAY_RC=$?
LY_NEST=$(ly_wrap OPENWARRANT_IN_BATTERY=1 --); LY_NEST_RC=$?
if [[ "$LY_PINS" == $'tickets\t10-old.sh\tabout the older layout' \
    && $LY_UNK_RC -eq 2 && "$LY_UNK" == *"'99-nothing.sh' is not a plant file"* \
    && $LY_LAY_RC -eq 2 && "$LY_LAY" == *"not 'masters'"* \
    && $LY_NEST_RC -eq 2 && "$LY_NEST" == *plant-layouts.refused-nested* ]]; then
    ly_ok "pins are listed; bad input refused" "--pins names the one pinned file and its reason; an unknown plant, layout and a nested run exit 2"
else
    ly_fail "pins are listed; bad input refused" "pins '$LY_PINS' unknown $LY_UNK_RC layout $LY_LAY_RC nested $LY_NEST_RC"
fi

[[ -n "$LY_TMP" && "$LY_TMP" == */tmp.* ]] && /usr/bin/rm -rf "$LY_TMP"
unset LY_TMP LY_SESSION LY_W LY_K LY_P LY_BAD LY_BAD_RC LY_NR LY_NR_RC LY_REPO LY_BOTH LY_BOTH_RC LY_SEEN \
    LY_ONE LY_ONE_RC LY_DEAD LY_DEAD_RC LY_PINS LY_UNK LY_UNK_RC LY_LAY LY_LAY_RC LY_NEST LY_NEST_RC
