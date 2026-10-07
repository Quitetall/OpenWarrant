# shellcheck shell=bash
# OW-WAR-0148 M14 — the PR gate, `war check --pr <number>` (docs/PRESETS.md).
#
# A team repository and a fake `gh` on PATH that records every argv and
# answers `gh api <path>` from files named for the path; nothing reaches the
# network. The base commit's openwarrant.toml is served as the policy. Each
# claim is paired with a refusal:
#
#   1. an outsider's PR citing a title-only Warrant nobody approved is
#      refused (pr.not-official), naming the outsider and what would make it
#      pass; the same PR approved in review by a maintainer, at the head
#      commit, passes (pr.official).
#   2. an approving review by a disallowed role (a writer, for a vibe
#      Warrant) does not count, and the PR stays refused; a maintainer's
#      approval of an earlier commit does not count either.
#   3. a `war sign approve` by a roster key (a throwaway key in a throwaway
#      agent) makes the Warrant official and the PR passes; the same PR,
#      changing docs/authority/roles.toml, gets no credit for it.
#   4. GitHub not answering is UNKNOWN (severity unknown, exit 2), never a
#      pass; a PR that cites nothing is refused (pr.no-warrant), and one that
#      adds a Warrant cites it.
#   5. the policy is the base's: a PR whose own openwarrant.toml lets
#      outsiders vibe is still refused; a base whose preset is vibe does not
#      require it (pr.not-required).
#   6. the calls are exactly the reads named, in order; `--summary` appends
#      the Markdown summary and `--comment` posts it through `gh api -X POST`.

echo "== the PR gate (M14) =="
PG_T=$(mktemp -d)
PG_ROOT="$PG_T/repo"
PG_GH="$PG_T/gh"
PG_LOG="$PG_T/gh.log"
PG_WAR="$REPO_ROOT/${WAR#./}"
pg_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
pg_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
pg_py() { python3 -c "import json, sys; v = json.load(sys.stdin); print($1)" 2>/dev/null; }
pgw() { (cd "$PG_ROOT" && env -u OPENWARRANT_ACTOR -u GITHUB_REPOSITORY OPENWARRANT_NO_PROJECTS=1 \
    PATH="$PG_T/bin:$PATH" FAKE_GH_DIR="$PG_GH" FAKE_GH_LOG="$PG_LOG" "$PG_WAR" "$@" </dev/null); }
# The gate's verdict and rules, from its envelope: "<verdict> <exit> <rule,...>".
pg_gate() {
    pgw --json check --pr "$@" | pg_py '"%s %s %s" % (v["result"]["verdict"], v["exit_code"], ",".join(d["severity"] + ":" + d["rule"] for d in v["diagnostics"]))'
}

mkdir -p "$PG_T/bin" "$PG_GH" "$PG_ROOT"
cat > "$PG_T/bin/gh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$FAKE_GH_LOG"
if [[ " $* " == *" -X POST "* ]]; then
    printf '%s\n' "${!#}" > "$FAKE_GH_DIR/posted"
    exit 0
fi
key=$(printf '%s' "${!#}" | tr '/?=&' '____')
if [[ -f "$FAKE_GH_DIR/fail" ]] && grep -qxF -- "$key" "$FAKE_GH_DIR/fail"; then
    echo "error connecting to api.github.com" >&2; exit 1
fi
if [[ -f "$FAKE_GH_DIR/$key" ]]; then cat "$FAKE_GH_DIR/$key"; exit 0; fi
echo "gh: Not Found (HTTP 404)" >&2; exit 1
SH
chmod +x "$PG_T/bin/gh"

git -C "$PG_ROOT" init -q .
pgw init --team >/dev/null 2>&1
ssh-keygen -q -t ed25519 -N "" -C plant-ada -f "$PG_T/id_ada"
PG_PUB=$(cut -d' ' -f1,2 "$PG_T/id_ada.pub")
printf 'ada namespaces="oh.war/response,oh.war/dsse" %s\n' "$PG_PUB" > "$PG_ROOT/docs/authority/allowed_signers"
cat > "$PG_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Ada"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/142-pr-gate.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "ada"
ROLES
printf '\n[roles.roster]\nada = "maintain"\n' >> "$PG_ROOT/openwarrant.toml"
git -C "$PG_ROOT" add -A >/dev/null 2>&1
git -C "$PG_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm base >/dev/null 2>&1
PG_BASE=$(git -C "$PG_ROOT" rev-parse HEAD)
PG_HEAD=1111111111111111111111111111111111111111
PG_OLD=2222222222222222222222222222222222222222
PG_W=$(pgw --json create "Fix the login redirect" | pg_py 'v["result"]["id"]')

# The PR, its base policy and the people in it.
pg_pr() { # number body
    python3 -c 'import json, sys
print(json.dumps({"number": int(sys.argv[1]), "user": {"login": "eve"}, "body": sys.argv[2],
    "head": {"sha": sys.argv[3]}, "base": {"sha": sys.argv[4]}}))' "$1" "$2" "$PG_HEAD" "$PG_BASE" > "$PG_GH/repos_o_r_pulls_$1"
    printf '[]' > "$PG_GH/repos_o_r_pulls_$1_commits"
    printf '[]' > "$PG_GH/repos_o_r_pulls_$1_files"
    printf '[]' > "$PG_GH/repos_o_r_pulls_$1_reviews"
}
pg_review() { # number login commit
    printf '[{"user":{"login":"%s"},"state":"APPROVED","commit_id":"%s"}]' "$2" "$3" > "$PG_GH/repos_o_r_pulls_$1_reviews"
}
cp "$PG_ROOT/openwarrant.toml" "$PG_GH/repos_o_r_contents_openwarrant.toml_ref_$PG_BASE"
printf '{"permission":"none"}' > "$PG_GH/repos_o_r_collaborators_eve_permission"
printf '{"permission":"write","role_name":"maintain"}' > "$PG_GH/repos_o_r_collaborators_mia_permission"
printf '{"permission":"write","role_name":"write"}' > "$PG_GH/repos_o_r_collaborators_wil_permission"
pg_pr 7 "Fixes the redirect.

Warrant: $PG_W"

# ---- 1. an outsider, without and with a maintainer's approval -----------------------
: > "$PG_LOG"
PG_REFUSED=$(pgw check --pr 7 --repo o/r 2>&1); PG_REFUSED_RC=$?
PG_CALLS=$(cat "$PG_LOG")
PG_R1=$(pg_gate 7 --repo o/r)
pg_review 7 mia "$PG_HEAD"
PG_PASS=$(pg_gate 7 --repo o/r)
if [[ $PG_REFUSED_RC -eq 2 && "$PG_R1" == "refused 2 "*"error:pr.not-official"* ]] \
    && grep -qF '@eve (no role in o/r: an outsider)' <<<"$PG_REFUSED" \
    && grep -qF "war sign approve $PG_W --ssh-sign" <<<"$PG_REFUSED" \
    && grep -qF 'an approving review at the head commit by admin or maintain' <<<"$PG_REFUSED"; then
    pg_ok "an outsider's unofficial PR refused" "pr.not-official names @eve as an outsider and what would pass"
else
    pg_fail "an outsider's unofficial PR refused" "exit $PG_REFUSED_RC, '$PG_R1': $(grep -m1 pr.not-official <<<"$PG_REFUSED")"
fi
if [[ "$PG_PASS" == "pass 0 "*"pass:pr.official"* ]]; then
    pg_ok "a maintainer's approval passes it" "approved in review by @mia (maintain) at the head: pr.official"
else
    pg_fail "a maintainer's approval passes it" "'$PG_PASS'"
fi

# ---- 2. approvals that do not count ---------------------------------------------------
pg_review 7 wil "$PG_HEAD"
PG_WRITER=$(pg_gate 7 --repo o/r)
PG_WRITER_WHY=$(pgw check --pr 7 --repo o/r 2>&1)
pg_review 7 mia "$PG_OLD"
PG_STALE=$(pg_gate 7 --repo o/r)
PG_STALE_WHY=$(pgw check --pr 7 --repo o/r 2>&1)
if [[ "$PG_WRITER" == "refused 2 "*"error:pr.not-official"* && "$PG_STALE" == "refused 2 "* ]] \
    && grep -qF "@wil's approving review does not count: a write approves tested Warrants" <<<"$PG_WRITER_WHY" \
    && grep -qF "@mia's approval is of an earlier commit" <<<"$PG_STALE_WHY"; then
    pg_ok "a disallowed review is refused" "a writer's approval of a vibe Warrant, and an approval of an earlier commit, count for nothing"
else
    pg_fail "a disallowed review is refused" "writer '$PG_WRITER', stale '$PG_STALE'"
fi
printf '[]' > "$PG_GH/repos_o_r_pulls_7_reviews"

# ---- 3. a roster key's approval --------------------------------------------------------
eval "$(ssh-agent -s > "$PG_T/agent.env"; cat "$PG_T/agent.env")" >/dev/null
ssh-add -q "$PG_T/id_ada" 2>/dev/null
PG_KEYS=$(ssh-add -L 2>/dev/null)
if [[ "$(wc -l <<<"$PG_KEYS")" == "1" && "$PG_KEYS" == "$PG_PUB"* ]]; then
    pgw sign approve "$PG_W" --ssh-sign --as Ada >/dev/null 2>&1; PG_SIGN_RC=$?
else
    PG_SIGN_RC=9
fi
ssh-agent -k >/dev/null 2>&1 || true
unset SSH_AUTH_SOCK SSH_AGENT_PID
PG_SIGNED=$(pg_gate 7 --repo o/r)
printf '[{"filename":"docs/authority/roles.toml","status":"modified"}]' > "$PG_GH/repos_o_r_pulls_7_files"
PG_ROSTER=$(pg_gate 7 --repo o/r)
printf '[]' > "$PG_GH/repos_o_r_pulls_7_files"
if [[ $PG_SIGN_RC -eq 0 && "$PG_SIGNED" == "pass 0 "*"pass:pr.official"* ]]; then
    pg_ok "a roster key's approval passes it" "war sign approve by Ada (maintain, a throwaway key): official (signed)"
else
    pg_fail "a roster key's approval passes it" "sign exit $PG_SIGN_RC; gate '$PG_SIGNED'"
fi
if [[ "$PG_ROSTER" == "refused 2 "*"warn:pr.roster-changed"*"error:pr.not-official"* ]]; then
    pg_ok "a PR changing the roster loses it" "pr.roster-changed: the signed approval is not counted; pr.not-official"
else
    pg_fail "a PR changing the roster loses it" "'$PG_ROSTER'"
fi

# ---- 4. UNKNOWN, nothing cited, a Warrant added ----------------------------------------
printf 'repos_o_r_pulls_7\n' > "$PG_GH/fail"
PG_DOWN=$(pg_gate 7 --repo o/r)
printf 'repos_o_r_pulls_7_reviews\n' > "$PG_GH/fail"
command rm -f "$PG_ROOT/docs/authority/responses/$PG_W.approval.response.toml"*
PG_DOWN2=$(pg_gate 7 --repo o/r)
command rm -f "$PG_GH/fail"
if [[ "$PG_DOWN" == "unknown 2 unknown:pr.unknown" && "$PG_DOWN2" == "unknown 2 "*"unknown:pr.unknown"* \
    && "$PG_DOWN2" != *"pass:pr.official"* ]]; then
    pg_ok "GitHub not answering is UNKNOWN" "the PR unread: unknown; its reviews unread with nothing else official: unknown; never a pass"
else
    pg_fail "GitHub not answering is UNKNOWN" "'$PG_DOWN' | '$PG_DOWN2'"
fi
pg_pr 8 "A drive-by fix with no plan."
PG_NONE=$(pg_gate 8 --repo o/r)
pg_pr 9 "Adds its own plan."
PG_ADDED=$(pgw --json create "Guard the empty password" | pg_py 'v["result"]["id"]')
pgw add "$PG_ADDED" --test "true" >/dev/null 2>&1
printf '{"permission":"write","role_name":"write"}' > "$PG_GH/repos_o_r_collaborators_eve_permission"
printf '[{"filename":"docs/tickets/%s/manifest.toml","status":"added"}]' "$PG_ADDED" > "$PG_GH/repos_o_r_pulls_9_files"
PG_ADD=$(pgw --json check --pr 9 --repo o/r | pg_py '"%s %s" % (v["result"]["verdict"], ",".join(c["id"] + ":" + "+".join(c["from"]) for c in v["result"]["cited"]))')
printf '{"permission":"none"}' > "$PG_GH/repos_o_r_collaborators_eve_permission"
if [[ "$PG_NONE" == "refused 2 error:pr.no-warrant" && "$PG_ADD" == "pass $PG_ADDED:added" ]]; then
    pg_ok "a PR citing nothing is refused" "pr.no-warrant; a PR adding a tested Warrant by a writer cites it and passes"
else
    pg_fail "a PR citing nothing is refused" "none '$PG_NONE'; added '$PG_ADD'"
fi

# ---- 5. the base's policy ----------------------------------------------------------------
sed -i 's/^none = "formal"$/none = "vibe"/' "$PG_ROOT/openwarrant.toml"
PG_LOOSE=$(pg_gate 7 --repo o/r)
sed -i 's/^none = "vibe"$/none = "formal"/' "$PG_ROOT/openwarrant.toml"
sed 's/^name = "team"$/name = "vibe"/; s/^pr_requires_official = true$/pr_requires_official = false/' \
    "$PG_ROOT/openwarrant.toml" > "$PG_GH/repos_o_r_contents_openwarrant.toml_ref_$PG_BASE"
PG_VIBE=$(pg_gate 7 --repo o/r)
cp "$PG_ROOT/openwarrant.toml" "$PG_GH/repos_o_r_contents_openwarrant.toml_ref_$PG_BASE"
if [[ "$PG_LOOSE" == "refused 2 "*"error:pr.not-official"* && "$PG_VIBE" == "not_required 0 "*"pass:pr.not-required"* ]]; then
    pg_ok "the base's policy decides" "a PR's own none = vibe changes nothing; a vibe base does not require it"
else
    pg_fail "the base's policy decides" "loose '$PG_LOOSE'; vibe '$PG_VIBE'"
fi

# ---- 6. the calls, the summary, the comment ------------------------------------------------
PG_WANT="api repos/o/r/pulls/7
api -H Accept: application/vnd.github.raw repos/o/r/contents/openwarrant.toml?ref=$PG_BASE
api repos/o/r/collaborators/eve/permission
api --paginate repos/o/r/pulls/7/commits
api --paginate repos/o/r/pulls/7/files
api --paginate repos/o/r/pulls/7/reviews"
: > "$PG_LOG"; command rm -f "$PG_GH/posted"
pgw check --pr 7 --repo o/r --summary "$PG_T/summary.md" --comment >/dev/null 2>&1; PG_PUB_RC=$?
PG_POSTED=$(cat "$PG_GH/posted" 2>/dev/null)
PG_SUMMARY=$(cat "$PG_T/summary.md" 2>/dev/null)
if [[ "$PG_CALLS" == "$PG_WANT" && $PG_PUB_RC -eq 2 ]] \
    && grep -qF '### OpenWarrant: PR #7 is refused' <<<"$PG_SUMMARY" \
    && grep -qF 'api -X POST repos/o/r/issues/7/comments -f body=' <<<"$(cat "$PG_LOG")" \
    && grep -qF 'body=### OpenWarrant: PR #7 is refused' <<<"$PG_POSTED"; then
    pg_ok "reads exactly these; summary, comment" "six reads in order; --summary appended; --comment posted through gh"
else
    pg_fail "reads exactly these; summary, comment" "exit $PG_PUB_RC; calls: $(tr '\n' '|' <<<"$PG_CALLS")"
fi

unset -f pg_ok pg_fail pg_py pgw pg_gate pg_pr pg_review
command rm -rf "$PG_T"
