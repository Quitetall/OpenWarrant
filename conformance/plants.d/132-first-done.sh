# shellcheck shell=bash
# OW-WAR-0148 M12 — a newcomer's first done, using only the daily verbs.
#
# The M12 target was three commands, as in Beads (`war init`, `war create
# "x"`, `war done <id>`). Today it is four: `war done` ticks only what the
# caller has claimed (OW-WAR-0147 OBL-003, "done needs your own claim": a
# done that needed no claim would let two agents finish one item), so the
# path is init, create, claim, done. M12 changes placement and help only, so
# this plant records the path as it is, and is the place to tighten to three
# if that rule is ever relaxed.
#
# Each claim is paired with an observed refusal:
# - A fresh repository (git init, nothing else) reaches its first done in
#   four commands, each one a verb `war --help` lists, with no signature, no
#   question asked and nothing written outside the repository's own files.
# - Refused, by name, with nothing ticked: the done before the claim
#   (`ticket.not-claimed`) and a done of an id that names nothing
#   (`ticket.unknown`).

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
command rm -rf "$FD_T"
