# shellcheck shell=bash
# W1.6: each flag on the six is the command it folds, and a retiring spelling
# keeps working. Each claim beside its refusal:
#   a fold and the spelling it folds print the same bytes and exit the same;
#   the comparison tells two different commands apart;
#   a pipe sees no retirement line, and OPENWARRANT_DEPRECATIONS=warn does;
#   a current spelling never prints one;
#   a fold is refused where it would mean nothing.

echo "== the surface: folds and retirement (W1.6) =="
PLANT_ROOT=$(scratch_corpus SF)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
sf_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sf_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sf() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR -u OPENWARRANT_DEPRECATIONS OPENWARRANT_NO_PROJECTS=1 \
    "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
SF_T=$(mktemp -d)
SF_ID=$(sf create "Plant ticket" -i first 2>/dev/null | awk 'NR==1{print $1}')
sf compile >/dev/null 2>&1

sf_run() { local o="$1"; shift; sf --json "$@" >"$o.out" 2>"$o.err"; echo $? >"$o.rc"; }
sf_same() {
    sf_run "$SF_T/a" $1; sf_run "$SF_T/b" $2
    cmp -s "$SF_T/a.out" "$SF_T/b.out" && cmp -s "$SF_T/a.err" "$SF_T/b.err" && cmp -s "$SF_T/a.rc" "$SF_T/b.rc"
}
SF_BAD=""
sf_same "view next" "status --next" || SF_BAD="$SF_BAD next"
sf_same "next" "status --next" || SF_BAD="$SF_BAD top-next"
sf_same "view ready" "status --ready" || SF_BAD="$SF_BAD ready"
sf_same "view show $SF_ID" "status $SF_ID --view full_warrant" || SF_BAD="$SF_BAD show"
sf_same "view timeline" "status --timeline" || SF_BAD="$SF_BAD timeline"
if sf_same "view next" "status"; then SF_TEETH=no; else SF_TEETH=yes; fi
if [[ -z "$SF_BAD" && $SF_TEETH == yes ]]; then
    sf_ok "each fold is the same bytes" "next, ready, show, timeline; next and status told apart"
else
    sf_fail "each fold is the same bytes" "differ:$SF_BAD; teeth=$SF_TEETH"
fi

# The writers, on one item, each answering in the command's own envelope.
SF_A=$(sf --json start "$SF_ID" --claim 2>/dev/null); SF_AC=$?
SF_B=$(sf --json done "$SF_ID" --release 2>/dev/null); SF_BC=$?
sf admin claim "$SF_ID" >/dev/null 2>&1; SF_CC=$?
sf admin release "$SF_ID" >/dev/null 2>&1; SF_DC=$?
if [[ $SF_AC -eq 0 && $SF_BC -eq 0 && $SF_CC -eq 0 && $SF_DC -eq 0 ]] \
    && grep -q '"command": "claim"' <<<"$SF_A" && grep -q '"command": "release"' <<<"$SF_B"; then
    sf_ok "the writers fold too" "start --claim and done --release keep their envelopes; admin's still work"
else
    sf_fail "the writers fold too" "exit $SF_AC $SF_BC $SF_CC $SF_DC"
fi

SF_PIPE=$(sf next 2>&1 >/dev/null)
SF_WARN=$(OPENWARRANT_DEPRECATIONS=warn "$WAR" --root "$PLANT_ROOT" next 2>&1 >/dev/null </dev/null)
SF_CUR=$(OPENWARRANT_DEPRECATIONS=warn "$WAR" --root "$PLANT_ROOT" status --next 2>&1 >/dev/null </dev/null)
SF_JSON=$(OPENWARRANT_DEPRECATIONS=warn "$WAR" --root "$PLANT_ROOT" --json next 2>&1 >/dev/null </dev/null)
if [[ -z "$SF_PIPE" && -z "$SF_CUR" && -z "$SF_JSON" ]] && grep -q 'use `war status --next`' <<<"$SF_WARN"; then
    sf_ok "retiring is said only when asked" "a pipe and --json see nothing; warn names the new spelling"
else
    sf_fail "retiring is said only when asked" "pipe='$SF_PIPE' warn='$SF_WARN' current='$SF_CUR' json='$SF_JSON'"
fi

sf status "$SF_ID" --next >/dev/null 2>&1; SF_1=$?
sf done --release >/dev/null 2>&1; SF_2=$?
sf done "$SF_ID" --reopen >/dev/null 2>&1; SF_3=$?
if [[ $SF_1 -eq 2 && $SF_2 -eq 2 && $SF_3 -eq 2 ]]; then
    sf_ok "a fold is refused where it means nothing" "exit 2 three times"
else
    sf_fail "a fold is refused where it means nothing" "exit $SF_1 $SF_2 $SF_3"
fi
command rm -rf "$SF_T"
