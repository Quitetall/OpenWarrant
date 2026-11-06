# shellcheck shell=bash
# OW-WAR-0148 M17 (decision 23; docs/SCORE.md): the opt-in CI floor, never
# drop a level.
#
# A scratch program (SF) whose base commit has every change citing a ticket
# and a ledger atom for every changed file; a head that adds a batch of
# uncited commits and deletes the atoms, lowering the level. Each claim is
# paired with a refusal:
# - `war check --floor <rev>` passes when the level holds (the head against
#   itself: PASS score.floor, exit 0), and refuses the head that lowers it
#   (ERROR score.floor naming both levels, exit 2); a base the clone does not
#   have is UNKNOWN, exit 2, never a pass.
# - through `war check --pr` (a fake `gh` on PATH; nothing reaches the
#   network) under the vibe preset, which does not require a Warrant: with
#   the base's openwarrant.toml lacking `[score] floor = true` the PR passes
#   as not required whatever the level does; with it, the same PR is refused
#   by score.floor, and the job summary names the floor.

echo "== the CI floor: a change never lowers the level (M17) =="
SF_T=$(mktemp -d)
SF_ROOT="$SF_T/repo"
SF_GH="$SF_T/gh"
SF_WAR="$REPO_ROOT/${WAR#./}"
sf_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sf_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sfw() { (cd "$SF_ROOT" && env -u OPENWARRANT_ACTOR -u GITHUB_REPOSITORY -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
    OPENWARRANT_NO_PROJECTS=1 PATH="$SF_T/bin:$PATH" FAKE_GH_DIR="$SF_GH" "$SF_WAR" "$@" </dev/null); }
sf_git() { git -C "$SF_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgSign=false "$@"; }
sf_py() { python3 -c "import json, sys; v = json.load(sys.stdin); print($1)" 2>/dev/null; }

mkdir -p "$SF_T/bin" "$SF_GH" "$SF_ROOT"
cat > "$SF_T/bin/gh" <<'SH'
#!/usr/bin/env bash
key=$(printf '%s' "${!#}" | tr '/?=&' '____')
if [[ -f "$FAKE_GH_DIR/$key" ]]; then cat "$FAKE_GH_DIR/$key"; exit 0; fi
echo "gh: Not Found (HTTP 404)" >&2; exit 1
SH
chmod +x "$SF_T/bin/gh"

git -C "$SF_ROOT" init -q .
sfw init --vibe >/dev/null 2>&1
SF_ID=$(sfw --json create "Tidy the parser" | sf_py 'v["result"]["id"]')
sf_git add -A >/dev/null 2>&1
sf_git commit -qm "Start" -m "Warrant: $SF_ID" >/dev/null 2>&1
mkdir -p "$SF_ROOT/src"
for i in 1 2 3 4; do
    printf 'fn f%s() {}\n' "$i" > "$SF_ROOT/src/f$i.rs"
    sf_git add -A >/dev/null 2>&1
    sf_git commit -qm "Parser step $i" -m "Warrant: $SF_ID" >/dev/null 2>&1
done
sfw admin ledger record "$(git -C "$SF_ROOT" rev-list --max-parents=0 HEAD)..HEAD" >/dev/null 2>&1
sfw admin ledger record "$(git -C "$SF_ROOT" rev-list --max-parents=0 HEAD)" >/dev/null 2>&1
sf_git add -A >/dev/null 2>&1
sf_git commit -qm "Ledger" -m "Warrant: $SF_ID" >/dev/null 2>&1
SF_BASE=$(git -C "$SF_ROOT" rev-parse HEAD)
# The head: twelve uncited commits on new files, and the atoms gone.
for i in $(seq 1 12); do
    printf 'fn g%s() {}\n' "$i" > "$SF_ROOT/src/g$i.rs"
    sf_git add -A >/dev/null 2>&1
    sf_git commit -qm "Quick fix $i" >/dev/null 2>&1
done
sf_git rm -rq docs/ledger >/dev/null 2>&1
sf_git commit -qm "Drop the ledger" >/dev/null 2>&1
SF_LB=$(sfw --json admin score --at "$SF_BASE" | sf_py 'v["result"]["level"]["number"]')
SF_LH=$(sfw --json admin score | sf_py 'v["result"]["level"]["number"]')
[[ "$SF_LB" =~ ^[0-9]$ && "$SF_LH" =~ ^[0-9]$ && $SF_LH -lt $SF_LB ]] \
    || { printf 'PLANT SETUP FAILED: the head does not lower the level (base %s, head %s)\n' "$SF_LB" "$SF_LH" >&2; exit 9; }

# --- war check --floor -----------------------------------------------------------------------
SF_HOLD=$(sfw check --floor HEAD 2>&1); SF_HOLD_RC=$?
SF_DROP=$(sfw check --floor "$SF_BASE" 2>&1); SF_DROP_RC=$?
SF_GONE=$(sfw --json check --floor 0123456789abcdef0123456789abcdef01234567 \
    | sf_py '"%s %s:%s" % (v["result"]["verdict"], v["diagnostics"][0]["severity"], v["diagnostics"][0]["rule"])'); SF_GONE_RC=$?
SF_WT=$(git -C "$SF_ROOT" worktree list | wc -l)
if [[ $SF_HOLD_RC -eq 0 ]] && line_has -F 'score.floor' -E '^PASS' <<<"$SF_HOLD"; then
    sf_ok "a level that holds passes" "--floor HEAD: PASS score.floor, exit 0"
else
    sf_fail "a level that holds passes" "exit $SF_HOLD_RC: $SF_HOLD"
fi
if [[ $SF_DROP_RC -eq 2 && "$SF_GONE" == "unknown unknown:score.floor" && $SF_WT -eq 1 ]] \
    && line_has -F 'score.floor' -F "lowers the level from $SF_LB" <<<"$SF_DROP"; then
    sf_ok "a change lowering the level is refused" "--floor base: level $SF_LB -> $SF_LH, ERROR score.floor, exit 2; an absent base is UNKNOWN; no worktree left"
else
    sf_fail "a change lowering the level is refused" "exit $SF_DROP_RC: $(head -c 300 <<<"$SF_DROP"); absent base '$SF_GONE'; worktrees $SF_WT"
fi

# --- through the PR gate ----------------------------------------------------------------------
SF_HEAD=$(git -C "$SF_ROOT" rev-parse HEAD)
python3 -c 'import json, sys
print(json.dumps({"number": 9, "user": {"login": "eve"}, "body": "Quick fixes.",
    "head": {"sha": sys.argv[1]}, "base": {"sha": sys.argv[2]}}))' "$SF_HEAD" "$SF_BASE" > "$SF_GH/repos_o_r_pulls_9"
for f in commits files reviews; do printf '[]' > "$SF_GH/repos_o_r_pulls_9_$f"; done
printf '{"permission":"none"}' > "$SF_GH/repos_o_r_collaborators_eve_permission"
git -C "$SF_ROOT" show "$SF_BASE:openwarrant.toml" > "$SF_GH/repos_o_r_contents_openwarrant.toml_ref_$SF_BASE"
sf_gate() { sfw --json check --pr 9 --repo o/r "$@" | sf_py '"%s %s %s" % (v["result"]["verdict"], v["exit_code"], ",".join(d["severity"] + ":" + d["rule"] for d in v["diagnostics"]))'; }
SF_OFF=$(sf_gate)
printf '\n[score]\nfloor = true\n' >> "$SF_GH/repos_o_r_contents_openwarrant.toml_ref_$SF_BASE"
SF_ON=$(sf_gate --summary "$SF_T/summary.md")
SF_SUMMARY=$(cat "$SF_T/summary.md" 2>/dev/null)
if [[ "$SF_OFF" == "not_required 0"* && "$SF_OFF" != *"score.floor"* ]]; then
    sf_ok "without the base's floor, no floor" "vibe PR #9: not_required, exit 0, no score.floor"
else
    sf_fail "without the base's floor, no floor" "'$SF_OFF'"
fi
if [[ "$SF_ON" == "refused 2 "*"error:score.floor"* ]] && line_has -F 'Score floor' -F 'refused' <<<"$SF_SUMMARY"; then
    sf_ok "the base's floor refuses the PR" "[score] floor = true at the base: PR #9 refused by score.floor; the summary names it"
else
    sf_fail "the base's floor refuses the PR" "'$SF_ON'; summary: $(head -c 300 <<<"$SF_SUMMARY")"
fi

command rm -rf "$SF_T"
