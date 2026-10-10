# shellcheck shell=bash
# OW-WAR-0148 M14 — roles, the roster and official Warrants, from a checkout
# (docs/PRESETS.md).
#
# A team repository whose register (docs/authority/roles.toml) names two
# people: Ada, whose principal `ada` is a maintainer in [roles.roster], and
# Wes, whose `wes` is a writer. Ada's key is generated here and held by a
# throwaway ssh-agent asserted to hold only it; nothing reaches the
# caller's agent, and nothing signs for anyone. Each claim is paired with a
# refusal:
#
#   1. role mapping: a title-only Warrant Ada made is official (author:
#      maintain may make a vibe Warrant alone); the same by Wes is not
#      (write needs a test) until it carries one. A Warrant an agent made is
#      not established here, and the GitHub review is UNKNOWN, never a pass.
#   2. an approval: `war sign approve <id>` records the request and `war
#      next` lists it for a person; Wes's signature is refused
#      (approve.no-eligible-approver: a writer approves tested or more
#      formal); Ada's verifies, and the Warrant reads official (signed).
#      Changing the plan makes the approval stale: it no longer counts.
#   3. a roster role that is not a role is refused by `war check`
#      (preset.config), naming the principal.

echo "== roles, the roster and official Warrants (M14) =="
RL_T=$(mktemp -d)
RL_ROOT="$RL_T/repo"
RL_WAR="$REPO_ROOT/${WAR#./}"
rl_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rl_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
rlw() { (cd "$RL_ROOT" && env -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 "$RL_WAR" "$@" </dev/null); }
rl_py() { python3 -c "import json, sys; v = json.load(sys.stdin); print($1)" 2>/dev/null; }
rl_new() { rlw --json create "$1" --as "$2" | rl_py 'v["result"]["id"]'; }
rl_official() { rlw --json show "$1" | rl_py '"%s %s" % (v["result"]["official"]["official"], v["result"]["official"].get("basis"))'; }

mkdir -p "$RL_ROOT"
git -C "$RL_ROOT" init -q .
rlw init --team >/dev/null 2>&1
ssh-keygen -q -t ed25519 -N "" -C plant-ada -f "$RL_T/id_ada"
RL_PUB=$(cut -d' ' -f1,2 "$RL_T/id_ada.pub")
printf 'ada namespaces="oh.war/response,oh.war/dsse" %s\n' "$RL_PUB" > "$RL_ROOT/docs/authority/allowed_signers"
cat > "$RL_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Ada"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/141-roles.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "ada"

[[assignment]]
actor = "Wes"
actor_kind = "human"
roles = ["authorizer"]
assigned_by = "conformance/plants.d/141-roles.sh"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "wes"
ROLES
printf '\n[roles.roster]\nada = "maintain"\nwes = "write"\n' >> "$RL_ROOT/openwarrant.toml"

# ---- 1. role mapping --------------------------------------------------------------
RL_ROSTER=$(rlw --json admin preset | rl_py '",".join(k + "=" + r for k, r in sorted(v["result"]["roster"].items()))')
RL_ADA=$(rl_new "Tidy the login page" Ada)
RL_WES=$(rl_new "Cache the session" Wes)
RL_AGENT=$(rl_new "Fix the redirect" claude)
RL_ADA_OFF=$(rl_official "$RL_ADA")
RL_WES_OFF=$(rl_official "$RL_WES")
rlw add "$RL_WES" --test "true" >/dev/null 2>&1
RL_WES_TESTED=$(rl_official "$RL_WES")
RL_AGENT_SHOW=$(rlw show "$RL_AGENT" 2>&1)
RL_AGENT_OFF=$(rl_official "$RL_AGENT")
if [[ "$RL_ROSTER" == "ada=maintain,wes=write" && "$RL_ADA_OFF" == "True author" \
    && "$RL_WES_TESTED" == "True author" ]]; then
    rl_ok "a role allows its kind alone" "roster ada=maintain, wes=write; Ada's vibe and Wes's tested Warrants official (author)"
else
    rl_fail "a role allows its kind alone" "roster '$RL_ROSTER'; Ada '$RL_ADA_OFF'; Wes tested '$RL_WES_TESTED'"
fi
if [[ "$RL_WES_OFF" == "None None" && "$RL_AGENT_OFF" == "None None" ]] \
    && grep -qF 'official: not established here' <<<"$RL_AGENT_SHOW" \
    && grep -qF 'GitHub review: unknown here' <<<"$RL_AGENT_SHOW" \
    && grep -qF "war sign approve $RL_AGENT --ssh-sign" <<<"$RL_AGENT_SHOW"; then
    rl_ok "a role below the kind is not" "Wes's vibe and the agent's Warrant not established; the review UNKNOWN; the approval named"
else
    rl_fail "a role below the kind is not" "Wes '$RL_WES_OFF', agent '$RL_AGENT_OFF': $(grep -A3 '## Official' <<<"$RL_AGENT_SHOW" | tr '\n' '|')"
fi

# ---- 2. an approval -------------------------------------------------------------------
RL_REQ=$(rlw sign approve "$RL_AGENT" 2>&1); RL_REQ_RC=$?
RL_NEXT=$(rlw next 2>&1)
RL_INBOX=$(rlw --json sign inbox | rl_py '",".join(a["warrant"] for a in v["result"].get("approvals", []))')
RL_WES_SIGN=$(rlw sign approve "$RL_AGENT" --ssh-sign --as Wes 2>&1); RL_WES_RC=$?
RL_RESPONSE="$RL_ROOT/docs/authority/responses/$RL_AGENT.approval.response.toml"
RL_LEFT_BY_WES=$([[ -e "$RL_RESPONSE" ]] && echo yes || echo no)
eval "$(ssh-agent -s > "$RL_T/agent.env"; cat "$RL_T/agent.env")" >/dev/null
ssh-add -q "$RL_T/id_ada" 2>/dev/null
RL_KEYS=$(ssh-add -L 2>/dev/null)
if [[ "$(wc -l <<<"$RL_KEYS")" == "1" && "$RL_KEYS" == "$RL_PUB"* ]]; then
    RL_SIGN=$(rlw sign approve "$RL_AGENT" --ssh-sign --as Ada 2>&1); RL_SIGN_RC=$?
else
    RL_SIGN="the throwaway agent holds more than the plant's key: $RL_KEYS"; RL_SIGN_RC=9
fi
ssh-agent -k >/dev/null 2>&1 || true
unset SSH_AUTH_SOCK SSH_AGENT_PID
RL_SIGNED_OFF=$(rl_official "$RL_AGENT")
RL_NEXT_AFTER=$(rlw next 2>&1)
if [[ $RL_REQ_RC -eq 0 && $RL_SIGN_RC -eq 0 && "$RL_SIGNED_OFF" == "True signed" && "$RL_INBOX" == "$RL_AGENT" ]] \
    && grep -qF "approval requested for $RL_AGENT" <<<"$RL_REQ" \
    && line_has -F "$RL_AGENT" -F 'war sign approve' <<<"$RL_NEXT" \
    && grep -qF "approved by Ada (ada, maintain)" <<<"$RL_SIGN" \
    && ! grep -qF 'war sign approve' <<<"$RL_NEXT_AFTER"; then
    rl_ok "an approval makes it official" "requested, listed by war next and the inbox; Ada's signature verifies; official (signed)"
else
    rl_fail "an approval makes it official" "request $RL_REQ_RC, sign $RL_SIGN_RC, now '$RL_SIGNED_OFF', inbox '$RL_INBOX': $(head -2 <<<"$RL_SIGN" | tr '\n' '|')"
fi
if [[ $RL_WES_RC -eq 2 && "$RL_LEFT_BY_WES" == "no" ]] \
    && grep -qF 'approve.no-eligible-approver' <<<"$RL_WES_SIGN"; then
    rl_ok "a writer cannot approve a vibe one" "approve.no-eligible-approver; no response written, no key asked"
else
    rl_fail "a writer cannot approve a vibe one" "exit $RL_WES_RC, response left: $RL_LEFT_BY_WES: $RL_WES_SIGN"
fi
rlw add "$RL_AGENT" "Also cover the logout redirect" >/dev/null 2>&1
RL_STALE=$(rlw show "$RL_AGENT" 2>&1)
RL_STALE_OFF=$(rl_official "$RL_AGENT")
if [[ "$RL_STALE_OFF" == "None None" ]] && grep -qF 'is stale' <<<"$RL_STALE"; then
    rl_ok "a changed plan unseats the approval" "the approval binds the plan as it stood; now stale, not established"
else
    rl_fail "a changed plan unseats the approval" "'$RL_STALE_OFF': $(grep -A2 '## Official' <<<"$RL_STALE" | tr '\n' '|')"
fi

# ---- 3. a roster role that is not a role ------------------------------------------------
sed -i 's/^wes = "write"$/wes = "owner"/' "$RL_ROOT/openwarrant.toml"
RL_CHECK=$(rlw check 2>&1); RL_CHECK_RC=$?
if [[ $RL_CHECK_RC -eq 2 ]] && grep -qF 'preset.config' <<<"$RL_CHECK" \
    && grep -qF '[roles.roster] wes = "owner"' <<<"$RL_CHECK"; then
    rl_ok "an unknown roster role is refused" "war check: preset.config names [roles.roster] wes"
else
    rl_fail "an unknown roster role is refused" "exit $RL_CHECK_RC: $(grep -m1 -F preset <<<"$RL_CHECK")"
fi

unset -f rl_ok rl_fail rlw rl_py rl_new rl_official
command rm -rf "$RL_T"
