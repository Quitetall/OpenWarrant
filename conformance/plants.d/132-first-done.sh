# shellcheck shell=bash
# OW-WAR-0148 M12 — a newcomer's first done, using only the daily verbs.
#
# The M12 target was three commands, as in Beads (`war init`, `war create
# "x"`, `war done <id>`). Without a preset it is four: `war done` ticks only
# what the caller has claimed (OW-WAR-0147 OBL-003, "done needs your own
# claim": a done that needed no claim would let two agents finish one item),
# so the path is init, create, claim, done. M14: under the vibe preset (`war
# init --vibe`) it is three, because `war done` on an item nobody holds
# claims it first; the journal records the claim and the tick.
#
# Each claim is paired with an observed refusal:
# - A fresh repository (git init, nothing else) reaches its first done in
#   four commands, each one a verb `war --help` lists, with no signature, no
#   question asked and nothing written outside the repository's own files.
# - Refused, by name, with nothing ticked: the done before the claim
#   (`ticket.not-claimed`) and a done of an id that names nothing
#   (`ticket.unknown`).
# - M14: under `--vibe`, the first done is three daily verbs, and the
#   journal holds `ticket.claimed` then `ticket.item_done`. Refused even
#   there, by name: a done of an item another agent holds
#   (`ticket.claimed-by-other`) and of an item that waits on another
#   (`ticket.blocked`); neither is claimed or ticked.

echo "== the small surface: first done (M12) =="
FD_T=$(mktemp -d)
FD_ROOT="$FD_T/repo"
FD_WAR="$REPO_ROOT/${WAR#./}"
fd_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fd_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# No terminal to ask on, no inherited actor, no key: a newcomer's shell.
fdw() { (cd "$FD_ROOT" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
    OPENWARRANT_NO_PROJECTS=1 "$FD_WAR" "$@" </dev/null); }
mkdir -p "$FD_ROOT"
git -C "$FD_ROOT" init -q .

FD_HELP=$("$FD_WAR" --help 2>/dev/null)
FD_DAILY=$(awk '$0 == "Commands:" {f=1; next} /^[^ ]/ || /^$/ {f=0} f && /^  [a-z]/ {print $1}' <<<"$FD_HELP" | tr '\n' ' ')
# Every command the newcomer types, by its first word.
FD_STEPS=()

# 1. init  2. create  3. claim  4. done
FD_STEPS+=(init); fdw init >/dev/null 2>&1; FD_I=$?
FD_STEPS+=(create); FD_OUT=$(fdw create --json "Fix the login redirect" 2>/dev/null); FD_C=$?
FD_ID=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["result"]["id"])' "$FD_OUT" 2>/dev/null)
# The refusal half first, on the same Warrant: done before any claim.
FD_ERR=$(fdw done "$FD_ID" 2>&1 >/dev/null); FD_R1=$?
FD_ERR2=$(fdw done t-ffff 2>&1 >/dev/null); FD_R2=$?
FD_STATE0=$(fdw --json show "$FD_ID" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["ticket"]["state"])' 2>/dev/null)
FD_STEPS+=(claim); fdw claim "$FD_ID" >/dev/null 2>&1; FD_K=$?
FD_STEPS+=(done); fdw done "$FD_ID" >/dev/null 2>&1; FD_D=$?
FD_STATE=$(fdw --json show "$FD_ID" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["ticket"]["state"])' 2>/dev/null)

FD_WORDS=""; FD_ALL_DAILY=1
for w in "${FD_STEPS[@]}"; do
    FD_WORDS="$FD_WORDS $w"
    [[ " $FD_DAILY " == *" $w "* ]] || FD_ALL_DAILY=0
done
FD_SIGNED=$(find "$FD_ROOT/docs" \( -name 'authorization*' -o -name 'resolution*' -o -name '*.sig' \) 2>/dev/null | wc -l)
if [[ $FD_I -eq 0 && $FD_C -eq 0 && $FD_K -eq 0 && $FD_D -eq 0 && "$FD_STATE" == "done" \
    && ${#FD_STEPS[@]} -le 4 && $FD_ALL_DAILY -eq 1 && $FD_SIGNED -eq 0 ]]; then
    fd_ok "first done in four daily verbs" "${#FD_STEPS[@]} commands:$FD_WORDS; $FD_ID done, nothing signed"
else
    fd_fail "first done in four daily verbs" "init $FD_I create $FD_C claim $FD_K done $FD_D state '$FD_STATE' steps ${#FD_STEPS[@]}:$FD_WORDS daily=$FD_ALL_DAILY signed=$FD_SIGNED"
fi

if [[ $FD_R1 -eq 2 && $FD_R2 -eq 2 && "$FD_STATE0" == "open" ]] \
    && grep -q 'ticket.not-claimed' <<<"$FD_ERR" && grep -q 'ticket.unknown' <<<"$FD_ERR2"; then
    fd_ok "done refuses unclaimed and unknown" "before the claim: ticket.not-claimed; t-ffff: ticket.unknown; nothing ticked"
else
    fd_fail "done refuses unclaimed and unknown" "exit $FD_R1/$FD_R2, state before claim '$FD_STATE0': $FD_ERR | $FD_ERR2"
fi
# ---- M14: three daily verbs under the vibe preset ------------------------------
FD_VROOT="$FD_T/vibe"
mkdir -p "$FD_VROOT"
git -C "$FD_VROOT" init -q .
fdv() { (cd "$FD_VROOT" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
    OPENWARRANT_NO_PROJECTS=1 "$FD_WAR" "$@" </dev/null); }
FD_VSTEPS=()
FD_VSTEPS+=(init); fdv init --vibe >/dev/null 2>&1; FD_VI=$?
FD_VSTEPS+=(create); FD_VOUT=$(fdv create --json "Fix the login redirect" 2>/dev/null); FD_VC=$?
FD_VID=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["result"]["id"])' "$FD_VOUT" 2>/dev/null)
FD_VSTEPS+=(done); FD_VDONE=$(fdv done "$FD_VID" 2>&1); FD_VD=$?
FD_VSTATE=$(fdv --json show "$FD_VID" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["ticket"]["state"])' 2>/dev/null)
FD_VEVENTS=$(python3 -c 'import json,sys; print(" ".join(json.loads(l)["type"] for l in open(sys.argv[1])))' \
    "$FD_VROOT/docs/tickets/$FD_VID/journal.jsonl" 2>/dev/null)
FD_VALL=1
for w in "${FD_VSTEPS[@]}"; do [[ " $FD_DAILY " == *" $w "* ]] || FD_VALL=0; done
if [[ $FD_VI -eq 0 && $FD_VC -eq 0 && $FD_VD -eq 0 && "$FD_VSTATE" == "done" && ${#FD_VSTEPS[@]} -eq 3 \
    && $FD_VALL -eq 1 && "$FD_VEVENTS" == "ticket.created ticket.claimed ticket.item_done" ]] \
    && grep -qF 'claimed '"$FD_VID"' first: under the vibe preset' <<<"$FD_VDONE"; then
    fd_ok "vibe: first done in three verbs" "init --vibe, create, done; journal: $FD_VEVENTS"
else
    fd_fail "vibe: first done in three verbs" "init $FD_VI create $FD_VC done $FD_VD state '$FD_VSTATE' events '$FD_VEVENTS': $(head -2 <<<"$FD_VDONE" | tr '\n' '|')"
fi
# Refusals that hold under vibe: another agent's item, and a blocked one.
FD_VOUT=$(fdv create --json "Two steps" -i "First" -i "Second" 2>/dev/null)
FD_VT=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["result"]["id"])' "$FD_VOUT" 2>/dev/null)
FD_VI1=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["result"]["items"][0]["id"])' "$FD_VOUT" 2>/dev/null)
FD_VI2=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["result"]["items"][1]["id"])' "$FD_VOUT" 2>/dev/null)
fdv add "$FD_VT" "Third, after the first" --after "$FD_VI1" >/dev/null 2>&1
FD_VI3=$(fdv --json show "$FD_VT" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["items"][2]["id"])' 2>/dev/null)
fdv claim "$FD_VT/$FD_VI2" --as other-agent >/dev/null 2>&1
FD_VHELD=$(fdv done "$FD_VT/$FD_VI2" 2>&1); FD_VHELD_RC=$?
FD_VBLOCKED=$(fdv done "$FD_VT/$FD_VI3" 2>&1); FD_VBLOCKED_RC=$?
FD_VCL=$(cat "$FD_VROOT/docs/tickets/$FD_VT/atoms/15-checklist.md" 2>/dev/null)
FD_VCLAIMS=$(grep -c 'ticket.claimed' "$FD_VROOT/docs/tickets/$FD_VT/journal.jsonl")
if [[ $FD_VHELD_RC -eq 2 && $FD_VBLOCKED_RC -eq 2 && "$FD_VCLAIMS" == "1" ]] \
    && grep -qF 'ticket.claimed-by-other' <<<"$FD_VHELD" && grep -qF 'ticket.blocked' <<<"$FD_VBLOCKED" \
    && ! grep -q '\[x\]' <<<"$FD_VCL"; then
    fd_ok "vibe still refuses held, blocked" "ticket.claimed-by-other, ticket.blocked; nothing claimed or ticked"
else
    fd_fail "vibe still refuses held, blocked" "held exit $FD_VHELD_RC, blocked exit $FD_VBLOCKED_RC, claims $FD_VCLAIMS: $FD_VHELD | $FD_VBLOCKED"
fi
unset -f fdv
command rm -rf "$FD_T"
