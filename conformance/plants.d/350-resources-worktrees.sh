# shellcheck shell=bash
# Resources in `war start` worktrees (OW-ADR-0037).
#
# Each claim beside its refusal:
# - Two worktrees started with `[resources.env]` share one build dir:
#   `{common}/openwarrant/build`, in `.openwarrant/env` and the Claude
#   settings `env`. Refused: a repository without `[resources]` gets neither.
# - A lane's items reuse one worktree; an item outside a lane gets its own.
# - The seed line appears with no seed, disappears once `war admin seed` ran,
#   and appears again ("stale") for a base the seed is not in.
# - `max_worktrees` refuses one more (`resources.worktree-limit`), naming the
#   idle managed worktree; the starts under it were accepted.

echo "== resources: worktrees share a build dir, lanes reuse one, a limit (OW-ADR-0037) =="
unset OPENWARRANT_SESSION_ID KATANA_SESSION_ID CODEX_THREAD_ID CODEX_SESSION_ID CLAUDE_CODE_SESSION_ID OPENCODE_SESSION_ID OPENCODE
RW_WAR="$REPO_ROOT/${WAR#./}"
rw_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rw_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
RW_T=$(mktemp -d) || { printf 'PLANT SETUP FAILED: mktemp\n' >&2; exit 9; }
SCRATCH_CORPORA+=("$RW_T")
RW_ENV=(env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u CLAUDECODE -u OPENWARRANT_VERBOSE OPENWARRANT_ACTOR=runner
    GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid GIT_COMMITTER_NAME=plant
    GIT_COMMITTER_EMAIL=plant@invalid OPENWARRANT_NO_UPDATE_CHECK=1 OPENWARRANT_NO_PROJECTS=1)
rw_in()  { local d=$1; shift; (cd "$d" && "${RW_ENV[@]}" "$@" </dev/null 2>&1); }
rw_repo() {
    mkdir -p "$1/src/api" "$1/src/db" && echo a > "$1/src/api/a" && echo b > "$1/src/db/b"
    rw_in "$1" git init -q -b main >/dev/null && rw_in "$1" "$RW_WAR" init >/dev/null
}
rw_id() { sed -n 's/^\([a-z][a-z]*-[0-9a-f][0-9a-f]*\)  .*/\1/p' <<<"$1" | head -n 1; }

# The refusal first: no [resources], no env.
RW_P="$RW_T/plain"
rw_repo "$RW_P" || { printf 'PLANT SETUP FAILED: plain repository\n' >&2; exit 9; }
RW_PI=$(rw_id "$(rw_in "$RW_P" "$RW_WAR" create "plain work")")
rw_in "$RW_P" git add -A >/dev/null && rw_in "$RW_P" git commit -qm plan >/dev/null
rw_in "$RW_P" "$RW_WAR" start --worktree "$RW_PI" --print >/dev/null
RW_PW="$RW_P/.openwarrant/worktrees/$RW_PI"

RW_R="$RW_T/repo"
rw_repo "$RW_R" || { printf 'PLANT SETUP FAILED: repository\n' >&2; exit 9; }
cat >> "$RW_R/openwarrant.toml" <<'EOF'

[resources]
seed = "true"
max_worktrees = 3
[resources.env]
CARGO_BUILD_BUILD_DIR = "{common}/openwarrant/build"
EOF
RW_LANE=$(rw_in "$RW_R" "$RW_WAR" lane new api --paths src/api | sed -n 's/^lane api \([^ ]*\) .*/\1/p')
rw_in "$RW_R" "$RW_WAR" lane new db --paths src/db >/dev/null
RW_A1=$(rw_id "$(rw_in "$RW_R" "$RW_WAR" create "api one" --lane api)")
RW_A2=$(rw_id "$(rw_in "$RW_R" "$RW_WAR" create "api two" --lane api)")
RW_D1=$(rw_id "$(rw_in "$RW_R" "$RW_WAR" create "db one" --lane db)")
RW_S=$(rw_id "$(rw_in "$RW_R" "$RW_WAR" create "solo work")")
RW_O=$(rw_id "$(rw_in "$RW_R" "$RW_WAR" create "old base work")")
rw_in "$RW_R" git add -A >/dev/null && rw_in "$RW_R" git commit -qm plan >/dev/null
RW_C1=$(rw_in "$RW_R" git rev-parse HEAD)
rw_in "$RW_R" git commit -q --allow-empty -m later >/dev/null
[[ -n "$RW_LANE" && -n "$RW_A1" && -n "$RW_A2" && -n "$RW_D1" && -n "$RW_S" && -n "$RW_O" ]] \
    || { printf 'PLANT SETUP FAILED: items (%s %s %s %s %s)\n' "$RW_A1" "$RW_A2" "$RW_D1" "$RW_S" "$RW_O" >&2; exit 9; }
RW_COMMON=$(realpath "$(rw_in "$RW_R" git rev-parse --path-format=absolute --git-common-dir)")
out1=$(rw_in "$RW_R" "$RW_WAR" start --lane api --worktree --print)
RW_W1="$RW_R/.openwarrant/worktrees/$RW_LANE"
"${RW_ENV[@]}" "$RW_WAR" --root "$RW_R" admin seed >/dev/null 2>&1; RW_SEED_RC=$?
out2=$(rw_in "$RW_R" "$RW_WAR" start --worktree "$RW_S" --print)
RW_W2="$RW_R/.openwarrant/worktrees/$RW_S"
out3=$(rw_in "$RW_R" "$RW_WAR" start --worktree "$RW_O" --base "$RW_C1" --print)

want="CARGO_BUILD_BUILD_DIR=$RW_COMMON/openwarrant/build"
e1=$(cat "$RW_W1/.openwarrant/env" 2>/dev/null); e2=$(cat "$RW_W2/.openwarrant/env" 2>/dev/null)
s1=$(grep -c "\"CARGO_BUILD_BUILD_DIR\": \"$RW_COMMON/openwarrant/build\"" "$RW_W1/.claude/settings.local.json" 2>/dev/null)
if [[ "$e1" == "$want" && "$e2" == "$want" && "$s1" == 1 && "$out1" == *"env: CARGO_BUILD_BUILD_DIR"* ]]; then
    rw_ok "two worktrees, one build dir" "$want"
else
    rw_fail "two worktrees, one build dir" "w1 '$e1' w2 '$e2' settings $s1"
fi
if [[ -d "$RW_PW" && ! -e "$RW_PW/.openwarrant/env" ]] \
    && ! grep -q CARGO_BUILD_BUILD_DIR "$RW_PW/.claude/settings.local.json" 2>/dev/null; then
    rw_ok "no [resources], no env" "neither .openwarrant/env nor settings env"
else
    rw_fail "no [resources], no env" "$(ls -a "$RW_PW/.openwarrant" 2>&1 | tr '\n' ' ')"
fi
if [[ "$out1" == *"seed: none yet"* && $RW_SEED_RC -eq 0 && "$out2" != *"seed:"* && "$out3" == *"seed: stale"* ]]; then
    rw_ok "the seed line comes and goes" "none yet; gone after war admin seed; stale for an older base"
else
    rw_fail "the seed line comes and goes" "seed rc $RW_SEED_RC; 1: $(grep seed <<<"$out1") 2: $(grep seed <<<"$out2") 3: $(grep seed <<<"$out3")"
fi

# The lane's second item resumes the lane's worktree; the solo item had its own.
(cd "$RW_W1" && echo x > src/api/x && "${RW_ENV[@]}" git add src/api/x && "${RW_ENV[@]}" git commit -qm work \
    && "${RW_ENV[@]}" "$RW_WAR" done "$RW_A1" --note ok) >/dev/null 2>&1
n_before=$(rw_in "$RW_R" git worktree list | wc -l)
out4=$(rw_in "$RW_R" "$RW_WAR" start --lane api --worktree --print)
n_after=$(rw_in "$RW_R" git worktree list | wc -l)
if [[ "$out4" == *"resumed $RW_A2"* && "$out4" == *"work in $RW_W1 "* && $n_before -eq $n_after && "$RW_W1" != "$RW_W2" ]]; then
    rw_ok "a lane reuses its worktree" "$RW_A2 in $RW_LANE's worktree; $n_after worktrees; solo has its own"
else
    rw_fail "a lane reuses its worktree" "$n_before -> $n_after: $(head -n 3 <<<"$out4")"
fi

# The limit: three managed; the solo one finished, so it is named idle.
(cd "$RW_W2" && "${RW_ENV[@]}" "$RW_WAR" done "$RW_S" --note ok) >/dev/null 2>&1
out5=$(rw_in "$RW_R" "$RW_WAR" start --lane db --worktree --print); rc5=$?
if [[ $rc5 -ne 0 && "$out5" == *"resources.worktree-limit"* && "$out5" == *"idle: "*".openwarrant/worktrees/$RW_S"* \
    && ! -e "$RW_R/.openwarrant/worktrees/$(basename "$RW_W1")-db" && -n "$out2" ]]; then
    rw_ok "max_worktrees refuses the 4th" "names idle .openwarrant/worktrees/$RW_S; 3 were accepted"
else
    rw_fail "max_worktrees refuses the 4th" "rc $rc5: $out5"
fi
