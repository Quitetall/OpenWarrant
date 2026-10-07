# shellcheck shell=bash
# OW-WAR-0148 M4 — states: a fixed kernel set plus declared refinements
# (OW-ADR-0031).
#
# One scratch program (SP) whose profile files declare states:
#   profiles/ticket.toml    `in_review` refines `in_progress`;
#   profiles/delivery.toml  `signed_off` refines `verified` (authenticated),
#                           `acknowledged` refines `draft`.
# SP-WAR-0001 is the scaffold's delivery Warrant (unsigned: its manifest pins
# no profile digest, so the profile edit moves nothing of it); one ticket has
# two items.
#
# Accepted: `war model` emits the fixed states that hold, each `computed` or
# `authenticated` with its RQ-032 facet, beside the builders' own states;
# `in_review` is entered on a claimed item, journaled, shown by `war show`
# and `war tickets`, and reads lapsed once the item is done; `acknowledged`
# is entered on the draft Warrant.
# Refused: a declared state refining no fixed state
# (profile.state-refines-unknown), one named like a fixed state
# (profile.state-collides), one its kind can never reach
# (profile.state-unreachable); `in_review` on an unclaimed item and on a done
# item, and `signed_off` on an obligation not verified
# (state.parent-not-holding), each with nothing written; a fixed state
# entered by hand (state.fixed); no authenticated state reads without its
# act. And a declared state meets no §56.1 requirement: the resolution dry
# run, the status checks and `war next` are byte-identical across entering
# one.

echo "== states: a fixed set plus declared refinements (OW-WAR-0148 M4) =="
SP_ROOT=$(scratch_corpus SP)
[[ -d "${SP_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
SP_TMP=$(mktemp -d)
SP_WAR="$REPO_ROOT/${WAR#./}"

if ! WAR="$SP_WAR" D="$SP_ROOT" R="$REPO_ROOT" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR bash -euo pipefail > "$SP_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
w() { "$WAR" --root . "$@"; }
mkdir -p profiles
cp "$R/profiles/ticket.toml" "$R/profiles/delivery.toml" profiles/
grep -q '^name = "in_review"$' profiles/ticket.toml || printf '\n[[states]]\nname = "in_review"\nrefines = "in_progress"\n' >> profiles/ticket.toml
printf '\n[[states]]\nname = "signed_off"\nrefines = "verified"\n\n[[states]]\nname = "acknowledged"\nrefines = "draft"\n' >> profiles/delivery.toml
w create "Ship the states" -i "write the code" -i "write the plant" >/dev/null
test -n "$(ls docs/tickets)"
w compile >/dev/null
g add -A; g commit -qm "profiles declaring states, and a ticket"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$SP_TMP/setup.log" >&2
    exit 9
fi

sp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sp_war()  { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$SP_WAR" --root "$SP_ROOT" "$@" </dev/null; }
sp_line() { grep -m1 -E "$1" <<<"$2"; }
sp_sha()  { sha256sum "$1" | cut -d' ' -f1; }
SP_W=SP-WAR-0001
SP_T=$(command ls "$SP_ROOT/docs/tickets" | head -1)
SP_ITEMS=$(sp_war show "$SP_T" --json 2>/dev/null | python3 -c 'import json,sys; print(" ".join(i["id"] for i in json.load(sys.stdin)["result"]["items"]))')
SP_I1=${SP_ITEMS%% *}
SP_I2=${SP_ITEMS##* }
[[ -n "$SP_T" && -n "$SP_I1" && "$SP_I1" != "$SP_I2" ]] || { printf 'PLANT SETUP FAILED: no ticket with two items (%s: %s)\n' "$SP_T" "$SP_ITEMS" >&2; exit 9; }
SP_J="$SP_ROOT/docs/tickets/$SP_T/journal.jsonl"
sp_states() {
    sp_war model --json 2>/dev/null | python3 -c '
import json, sys
for s in json.load(sys.stdin)["result"]["states"]:
    print(s["record"], s["kind"], s["value"], s.get("facet", "-"), s.get("refines", "-"), "lapsed" if s.get("lapsed") else "held")
'
}

# 1. The fixed states, in the model, beside the builders' own.
SP_M=$(sp_states)
if sp_line "^$SP_W computed draft phase - held$" "$SP_M" >/dev/null \
    && sp_line "^$SP_T/$SP_I1 computed open phase - held$" "$SP_M" >/dev/null \
    && sp_line "^$SP_T computed open phase - held$" "$SP_M" >/dev/null \
    && sp_line "^$SP_W rung draft - - held$" "$SP_M" >/dev/null \
    && sp_line "^$SP_W phase draft - - held$" "$SP_M" >/dev/null; then
    sp_ok "fixed states are in the model" "draft and open, computed, facet phase; rung and phase still beside them"
else
    sp_fail "fixed states are in the model" "$(head -c 400 <<<"$SP_M")"
fi
# Unsigned, nothing authenticated reads: no act is on record.
if ! sp_line ' authenticated ' "$SP_M" >/dev/null && ! sp_line "^$SP_W .* (authorized|resolved) " "$SP_M" >/dev/null; then
    sp_ok "no authenticated state unsigned" "no authorized, verified or resolved without its act"
else
    sp_fail "no authenticated state unsigned" "$(sp_line ' authenticated ' "$SP_M")"
fi

# 2. Declarations are checked, each refusal by its rule.
sp_profile() {
    printf 'schema = "oh.war/profile/v1"\nname = "lab"\nextends = "%s"\napproved = false\n%s\n[[states]]\nname = "%s"\nrefines = "%s"\n' \
        "$1" "$2" "$3" "$4" > "$SP_ROOT/profiles/lab.toml"
}
sp_refused_decl() {
    local name="$1" rule="$2" detail="$3" out status
    out=$(sp_war check 2>&1); status=$?
    command rm -f "$SP_ROOT/profiles/lab.toml"
    if [[ $status -ne 0 ]] && sp_line "$rule" "$out" >/dev/null && sp_line "$detail" "$out" >/dev/null; then
        sp_ok "$name" "refused: $rule"
    else
        sp_fail "$name" "exit $status: $(head -c 300 <<<"$out")"
    fi
}
sp_profile delivery "" in_review reviewing
sp_refused_decl "refines an unknown state" 'profile\.state-refines-unknown' 'refines "reviewing", which is not a fixed state'
sp_profile delivery "" verified in_progress
sp_refused_decl "named like a fixed state" 'profile\.state-collides' 'declared state `verified` is a fixed kernel state'
sp_profile delivery 'capabilities = ["structure", "links", "claims"]' signed_off verified
sp_refused_decl "refines a state it never reaches" 'profile\.state-unreachable' 'only the `verification` capability'
SP_OUT=$(sp_war check 2>&1); SP_STATUS=$?
if [[ $SP_STATUS -eq 0 ]] && ! sp_line 'profile\.state' "$SP_OUT" >/dev/null; then
    sp_ok "the declarations alone pass" "in_review, signed_off and acknowledged check clean"
else
    sp_fail "the declarations alone pass" "exit $SP_STATUS: $(sp_line 'profile\.|ERROR' "$SP_OUT")"
fi

# 3. in_review on an unclaimed item: refused, nothing written.
SP_J0=$(sp_sha "$SP_J")
SP_OUT=$(sp_war state "$SP_T/$SP_I1" in_review 2>&1); SP_STATUS=$?
if [[ $SP_STATUS -ne 0 && "$(sp_sha "$SP_J")" == "$SP_J0" ]] \
    && sp_line 'refused \(state\.parent-not-holding\).*`in_review` refines `in_progress`, which does not hold for it now \(it reads open\)' "$SP_OUT" >/dev/null; then
    sp_ok "in_review on an unclaimed item" "refused: state.parent-not-holding (it reads open); journal unchanged"
else
    sp_fail "in_review on an unclaimed item" "exit $SP_STATUS: $(head -c 300 <<<"$SP_OUT")"
fi
SP_OUT=$(sp_war state "$SP_W" verified 2>&1); SP_STATUS=$?
if [[ $SP_STATUS -ne 0 ]] && sp_line 'refused \(state\.fixed\).*`verified` is a fixed kernel state, authenticated' "$SP_OUT" >/dev/null; then
    sp_ok "a fixed state entered by hand" "refused: state.fixed"
else
    sp_fail "a fixed state entered by hand" "exit $SP_STATUS: $(head -c 300 <<<"$SP_OUT")"
fi

# 4. Claimed, it enters: journaled, in the model, shown.
SP_SHOW0=$(sp_war show "$SP_T" 2>&1)
sp_war claim "$SP_T/$SP_I1" >/dev/null 2>&1
SP_OUT=$(sp_war state "$SP_T/$SP_I1" in_review --note "PR open" 2>&1); SP_STATUS=$?
SP_EV=$(tail -1 "$SP_J")
SP_M=$(sp_states)
SP_SHOW=$(sp_war show "$SP_T" 2>&1)
SP_LS=$(sp_war tickets 2>&1)
if [[ $SP_STATUS -eq 0 ]] && sp_line '"type":"state\.entered"' "$SP_EV" >/dev/null \
    && sp_line "in_review" "$SP_EV" >/dev/null \
    && sp_line "^$SP_T/$SP_I1 declared in_review - in_progress held$" "$SP_M" >/dev/null \
    && sp_line "^$SP_T/$SP_I1 computed in_progress phase - held$" "$SP_M" >/dev/null \
    && sp_line "\($SP_I1\) — claimed by .* \[in_review\]$" "$SP_SHOW" >/dev/null \
    && sp_line "^$SP_T .*\[$SP_I1 in_review\]$" "$SP_LS" >/dev/null \
    && ! sp_line 'in_review' "$SP_SHOW0" >/dev/null; then
    sp_ok "in_review on a claimed item" "state.entered journaled; model: declared, refines in_progress; show and tickets name it"
else
    sp_fail "in_review on a claimed item" "exit $SP_STATUS: $(head -c 200 <<<"$SP_OUT") | $(sp_line "$SP_I1" "$SP_M") | $(sp_line "$SP_I1" "$SP_SHOW")"
fi
# The other item has none, and reads as before.
if ! sp_line "^$SP_T/$SP_I2 declared" "$SP_M" >/dev/null && sp_line "^- \[ \] write the plant \($SP_I2\)$" "$SP_SHOW" >/dev/null; then
    sp_ok "a record without one is unchanged" "$SP_I2: no declared state, its line as before"
else
    sp_fail "a record without one is unchanged" "$(sp_line "$SP_I2" "$SP_SHOW")"
fi

# 5. Done: in_review lapses, and cannot be entered again.
sp_war done "$SP_T/$SP_I1" >/dev/null 2>&1
SP_M=$(sp_states)
SP_SHOW=$(sp_war show "$SP_T" 2>&1)
SP_LS=$(sp_war tickets 2>&1)
if sp_line "^$SP_T/$SP_I1 declared in_review - in_progress lapsed$" "$SP_M" >/dev/null \
    && sp_line "^$SP_T/$SP_I1 computed done phase - held$" "$SP_M" >/dev/null \
    && sp_line "\($SP_I1\) — done by .*\[in_review, lapsed\]$" "$SP_SHOW" >/dev/null \
    && ! sp_line "in_review" "$SP_LS" >/dev/null; then
    sp_ok "in_review lapses when done" "model: lapsed; show: [in_review, lapsed]; tickets no longer lists it"
else
    sp_fail "in_review lapses when done" "$(sp_line "$SP_I1" "$SP_M") | $(sp_line "$SP_I1" "$SP_SHOW")"
fi
SP_J0=$(sp_sha "$SP_J")
SP_OUT=$(sp_war state "$SP_T/$SP_I1" in_review 2>&1); SP_STATUS=$?
if [[ $SP_STATUS -ne 0 && "$(sp_sha "$SP_J")" == "$SP_J0" ]] \
    && sp_line 'refused \(state\.parent-not-holding\).*\(it reads done\)' "$SP_OUT" >/dev/null; then
    sp_ok "in_review on a done item" "refused: state.parent-not-holding (it reads done); journal unchanged"
else
    sp_fail "in_review on a done item" "exit $SP_STATUS: $(head -c 300 <<<"$SP_OUT")"
fi

# 6. signed_off refines verified, an authenticated state: OBL-001 is not
#    verified, so it cannot be entered.
SP_WJ="$SP_ROOT/docs/warrants/$SP_W/journal.jsonl"
SP_J0=$(sp_sha "$SP_WJ")
SP_OUT=$(sp_war state "$SP_W/OBL-001" signed_off 2>&1); SP_STATUS=$?
if [[ $SP_STATUS -ne 0 && "$(sp_sha "$SP_WJ")" == "$SP_J0" ]] \
    && sp_line 'refused \(state\.parent-not-holding\).*`signed_off` refines `verified`.*`verified` is authenticated: `signed_off` can be entered only while `verified` already holds' "$SP_OUT" >/dev/null; then
    sp_ok "signed_off on an unverified obligation" "refused: state.parent-not-holding, naming the authenticated parent; journal unchanged"
else
    sp_fail "signed_off on an unverified obligation" "exit $SP_STATUS: $(head -c 300 <<<"$SP_OUT")"
fi

# 7. A declared state meets no §56.1 requirement: everything a resolution
#    reads is byte-identical across entering one.
sp_reads() {
    sp_war resolve --dry-run "$SP_W" --json 2>&1
    sp_war status --json 2>/dev/null | python3 -c '
import json, sys
w = [w for w in json.load(sys.stdin)["result"]["warrants"] if w["alias"] == sys.argv[1]][0]
print(json.dumps([w["rung"], w["checks"], w["unmet"], w["state"]], sort_keys=True))
' "$SP_W"
    sp_war next 2>&1
}
SP_R0=$(sp_reads)
SP_OUT=$(sp_war state "$SP_W" acknowledged --note "read it" 2>&1); SP_STATUS=$?
SP_R1=$(sp_reads)
SP_M=$(sp_states)
if [[ $SP_STATUS -eq 0 ]] && sp_line "^$SP_W declared acknowledged - draft held$" "$SP_M" >/dev/null \
    && sp_line '"type":"state\.entered"' "$(tail -1 "$SP_WJ")" >/dev/null; then
    sp_ok "acknowledged on the draft Warrant" "state.entered in its journal; model: declared, refines draft"
else
    sp_fail "acknowledged on the draft Warrant" "exit $SP_STATUS: $(head -c 300 <<<"$SP_OUT")"
fi
if [[ -n "$SP_R0" && "$SP_R0" == "$SP_R1" ]] && sp_line 'resolution\.requirement' "$SP_R1" >/dev/null \
    && sp_line '"unmet"|\["draft"' "$SP_R1" >/dev/null; then
    sp_ok "a declared state meets no §56.1 check" "resolve --dry-run, status checks/unmet/state and next byte-identical"
else
    sp_fail "a declared state meets no §56.1 check" "$(diff <(printf '%s\n' "$SP_R0") <(printf '%s\n' "$SP_R1") | head -6)"
fi

command rm -rf "$SP_TMP"
corpus_gone "$SP_ROOT"
unset SP_ROOT SP_TMP SP_WAR SP_W SP_T SP_ITEMS SP_I1 SP_I2 SP_J SP_WJ SP_J0 SP_M SP_OUT SP_STATUS SP_SHOW SP_SHOW0 SP_LS SP_EV SP_R0 SP_R1
