# shellcheck shell=bash
# One work order, one place (owner decision, 2026-11-07): the agent who reads
# one Warrant knows which worktree, branch and commit of which repository it
# works in, and nothing lets it work elsewhere. Each claim beside its refusal:
#   the brief's first line names the worktree, branch and base commit; a claim
#     made in a plain checkout names that checkout;
#   `done` and `note` from another worktree are refused naming the right one,
#     and from the claim's own worktree they pass; a read is never refused;
#   `war start` of work held from another worktree is refused naming it;
#   claiming for another actor in a checkout where you hold live work is
#     refused without `--here`;
#   a Warrant not committed (a single file, a docs/work/ master), or edited
#     since, is refused at dispatch with the command that fixes it;
#   work of another repository (`hpr:t-...`) names that repository and its pin
#     and is refused here; in its own repository it starts.

echo "== one work order, one place =="

unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID

op_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
op_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
op_has()  { [[ "$1" == *"$2"* ]]; }
OP_TMP=$(mktemp -d)
[[ -d "${OP_TMP:-}" ]] || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$OP_TMP") # The battery's own cleanup removes it.
OP_TMP=$(cd "$OP_TMP" && pwd -P)
op_git() { git -c user.email=plant@invalid -c user.name=plant -c commit.gpgsign=false "$@"; }
OP_BIN="$REPO_ROOT/target/debug/war"
# `war` in a directory, as an actor, with no harness session and no agent.
op_war() {
    local dir=$1 who=$2; shift 2
    (cd "$dir" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_NO_PROJECTS=1 \
        OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_ACTOR="$who" "$OP_BIN" "$@" 2>&1)
}
op_id() { sed -n 's/.*"id": "\(t-[0-9a-f]*\)".*/\1/p' | head -1; }
op_json() { python3 -c 'import json,sys
v=json.load(sys.stdin)
for k in sys.argv[1].split("."):
    v=v.get(k) if isinstance(v,dict) else None
print("" if v is None else v)' "$1"; }

OP_W="$OP_TMP/wmd"
OP_H="$OP_TMP/hpr"
for d in "$OP_W" "$OP_H"; do
    mkdir -p "$d" && op_git -C "$d" init -q -b main \
        || { printf 'PLANT SETUP FAILED: git init\n' >&2; exit 9; }
    OP_OUT=$(op_war "$d" lead init) || { printf 'PLANT SETUP FAILED: war init: %s\n' "$OP_OUT" >&2; exit 9; }
done
OP_HT=$(op_war "$OP_H" lead --json create "Upstream work" --item "a" | op_id)
op_git -C "$OP_H" add -A && op_git -C "$OP_H" commit -qm start \
    || { printf 'PLANT SETUP FAILED: hpr commit\n' >&2; exit 9; }
OP_PIN=$(op_git -C "$OP_H" rev-parse HEAD)
OP_A=$(op_war "$OP_W" lead --json create "Dispatched work" --item "a" | op_id)
OP_B=$(op_war "$OP_W" lead --json create "Local work" --item "b" | op_id)
printf '\n[repos.hpr]\npath = "../hpr"\npin = "%s"\n' "$OP_PIN" >> "$OP_W/openwarrant.toml"
op_git -C "$OP_W" add -A && op_git -C "$OP_W" commit -qm start \
    || { printf 'PLANT SETUP FAILED: wmd commit\n' >&2; exit 9; }
[[ -n "$OP_A" && -n "$OP_B" && -n "$OP_HT" ]] || { printf 'PLANT SETUP FAILED: no tickets\n' >&2; exit 9; }
OP_BASE=$(op_git -C "$OP_W" rev-parse HEAD)

# ---- dispatch: the brief's first line is the place ---------------------------
OP_OUT=$(op_war "$OP_W" lead --json start --worktree "$OP_A" --as worker --print)
OP_WT=$(printf '%s' "$OP_OUT" | op_json result.worktree)
OP_PLACE=$(printf '%s' "$OP_OUT" | op_json result.place)
OP_BRIEF1=$(printf '%s' "$OP_OUT" | op_json result.brief | head -1)
OP_WANT="work in $OP_WT (repo wmd, branch war-go/$OP_A, from ${OP_BASE:0:12})"
if [[ -d "$OP_WT" && "$OP_PLACE" == "$OP_WANT" && "$OP_BRIEF1" == "$OP_WANT" ]]; then
    op_ok "dispatch brief names its place" "$OP_PLACE"
else
    op_fail "dispatch brief names its place" "want [$OP_WANT] got place [$OP_PLACE] brief [$OP_BRIEF1]"
fi

# ---- done and note from another worktree: refused, naming the right one ------
OP_OUT=$(op_war "$OP_W" worker done "$OP_A" --note "from the wrong place"); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "work.other-worktree" && op_has "$OP_OUT" "cd $OP_WT"; then
    op_ok "done elsewhere is refused" "names $OP_WT"
else
    op_fail "done elsewhere is refused" "exit $OP_RC: $OP_OUT"
fi
OP_OUT=$(op_war "$OP_W" worker note "$OP_A" "from the wrong place"); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "work.other-worktree"; then
    op_ok "note elsewhere is refused" "work.other-worktree"
else
    op_fail "note elsewhere is refused" "exit $OP_RC: $OP_OUT"
fi
OP_OUT=$(op_war "$OP_W" worker start "$OP_A"); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "work.other-worktree" && op_has "$OP_OUT" "$OP_WT"; then
    op_ok "start elsewhere is refused" "names the claim's worktree"
else
    op_fail "start elsewhere is refused" "exit $OP_RC: $OP_OUT"
fi
OP_OUT=$(op_war "$OP_W" lead status "$OP_A"); OP_RC=$?
if [[ $OP_RC -eq 0 ]] && op_has "$OP_OUT" "$OP_WANT"; then
    op_ok "a read is never refused" "status shows the place"
else
    op_fail "a read is never refused" "exit $OP_RC: $OP_OUT"
fi
OP_OUT=$(op_war "$OP_WT" worker note "$OP_A" "from the right place"); OP_RC=$?
OP_OUT2=$(op_war "$OP_WT" worker done "$OP_A" --note "done where it was claimed"); OP_RC2=$?
if [[ $OP_RC -eq 0 && $OP_RC2 -eq 0 ]] && op_has "$OP_OUT2" "done $OP_A"; then
    op_ok "note and done in place pass" "$OP_A closed in its worktree"
else
    op_fail "note and done in place pass" "note exit $OP_RC, done exit $OP_RC2: $OP_OUT2"
fi

# ---- a plain checkout is a place too -----------------------------------------
OP_OUT=$(op_war "$OP_W" lead start "$OP_B"); OP_RC=$?
OP_FIRST=$(printf '%s\n' "$OP_OUT" | head -1)
if [[ $OP_RC -eq 0 && "$OP_FIRST" == "work in $OP_W (repo wmd, branch main, from ${OP_BASE:0:12})" ]]; then
    op_ok "plain start names its checkout" "$OP_FIRST"
else
    op_fail "plain start names its checkout" "exit $OP_RC: $OP_FIRST"
fi
op_git -C "$OP_W" worktree add -q "$OP_TMP/other" >/dev/null 2>&1
OP_OUT=$(op_war "$OP_TMP/other" lead done "$OP_B"); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "work.other-worktree" && op_has "$OP_OUT" "cd $OP_W\`"; then
    op_ok "done from a second worktree refused" "names the checkout"
else
    op_fail "done from a second worktree refused" "exit $OP_RC: $OP_OUT"
fi

# ---- dispatch into this checkout needs --here --------------------------------
OP_C=$(op_war "$OP_W" lead --json create "Third work" --item "c" | op_id)
op_git -C "$OP_W" add -A && op_git -C "$OP_W" commit -qm third >/dev/null
OP_OUT=$(op_war "$OP_W" lead admin claim "$OP_C" --as helper); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "dispatch.here" && op_has "$OP_OUT" "war start --worktree $OP_C --as helper"; then
    op_ok "claim for another here refused" "dispatch.here"
else
    op_fail "claim for another here refused" "exit $OP_RC: $OP_OUT"
fi
OP_OUT=$(op_war "$OP_W" lead admin claim "$OP_C" --as helper --here); OP_RC=$?
if [[ $OP_RC -eq 0 ]] && op_has "$OP_OUT" "claimed $OP_C"; then
    op_ok "--here claims it here on purpose" "claimed for helper"
else
    op_fail "--here claims it here on purpose" "exit $OP_RC: $OP_OUT"
fi

# ---- uncommitted work is refused at dispatch ---------------------------------
OP_D=$(op_war "$OP_W" lead --json create "Not committed" --item "d" | op_id)
OP_OUT=$(op_war "$OP_W" lead start --worktree "$OP_D" --as worker --print); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "go.start-uncommitted" && op_has "$OP_OUT" "git add docs/tickets/$OP_D.md"; then
    op_ok "uncommitted single file refused" "names the git add"
else
    op_fail "uncommitted single file refused" "exit $OP_RC: $OP_OUT"
fi
OP_M=$(op_war "$OP_W" lead plan new "A master" | sed -n 's/^created \(docs\/work\/[^ ]*\.md\).*/\1/p')
OP_MID=$(basename "$(dirname "$OP_M")")
OP_OUT=$(op_war "$OP_W" lead start --worktree "$OP_MID" --as worker --print); OP_RC=$?
if [[ -n "$OP_M" && $OP_RC -ne 0 ]] && op_has "$OP_OUT" "go.start-uncommitted" && op_has "$OP_OUT" "git add $OP_M"; then
    op_ok "uncommitted master refused" "names $OP_M"
else
    op_fail "uncommitted master refused" "[$OP_M] exit $OP_RC: $OP_OUT"
fi
op_git -C "$OP_W" add -A && op_git -C "$OP_W" commit -qm d >/dev/null
printf '\nEdited after the commit.\n' >> "$OP_W/docs/tickets/$OP_D.md"
OP_OUT=$(op_war "$OP_W" lead start --worktree "$OP_D" --as worker --print); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "go.start-uncommitted"; then
    op_ok "edited since commit refused" "go.start-uncommitted"
else
    op_fail "edited since commit refused" "exit $OP_RC: $OP_OUT"
fi
op_git -C "$OP_W" checkout -q -- "docs/tickets/$OP_D.md"
OP_OUT=$(op_war "$OP_W" lead start --worktree "$OP_D" --as worker --print); OP_RC=$?
if [[ $OP_RC -eq 0 ]] && op_has "$OP_OUT" "work in "; then
    op_ok "committed work dispatches" "$OP_D"
else
    op_fail "committed work dispatches" "exit $OP_RC: $OP_OUT"
fi

# ---- another repository's work: its repo and pin, and not here ---------------
OP_OUT=$(op_war "$OP_W" lead start "hpr:$OP_HT"); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "work.other-repo" && op_has "$OP_OUT" "hpr @ ${OP_PIN:0:12}" \
    && op_has "$OP_OUT" "cd $OP_H && war start --worktree $OP_HT"; then
    op_ok "cross-repo names repo and pin" "hpr @ ${OP_PIN:0:12}"
else
    op_fail "cross-repo names repo and pin" "exit $OP_RC: $OP_OUT"
fi
OP_OUT=$(op_war "$OP_W" lead start --worktree "hpr:$OP_HT" --as worker --print); OP_RC=$?
if [[ $OP_RC -ne 0 ]] && op_has "$OP_OUT" "work.other-repo"; then
    op_ok "cross-repo dispatch refused here" "work.other-repo"
else
    op_fail "cross-repo dispatch refused here" "exit $OP_RC: $OP_OUT"
fi
OP_OUT=$(op_war "$OP_H" lead start "$OP_HT"); OP_RC=$?
if [[ $OP_RC -eq 0 ]] && op_has "$(printf '%s\n' "$OP_OUT" | head -1)" "work in $OP_H (repo hpr,"; then
    op_ok "in its own repo it starts" "$OP_HT"
else
    op_fail "in its own repo it starts" "exit $OP_RC: $OP_OUT"
fi
