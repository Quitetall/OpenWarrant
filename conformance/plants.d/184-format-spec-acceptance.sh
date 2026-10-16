# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# A specification accepted record by record (FORMAT-1.0 §4.5, Appendix C.3),
# on a scratch program (SP); nothing in this repository moves.
#
# The act is signed here by a throwaway ed25519 key made in a scratch
# directory (`ssh-keygen -Y sign -f <key>`, no agent: SSH_AUTH_SOCK is
# removed), bound to a human authorizer in the scratch program's register.
#
# Accepted: an accept over every record of SPEC.md is believed; the
# specification reads `accepted`. One record is edited: it alone stops being
# accepted, the specification reads `partially_accepted`, `war next` lists
# the acceptance as a person's act, and `war impact` names the ticket citing
# the record.
# Refused, as the other half: the same act, now over a stale digest, is
# refused by name (spec.stale-digest), and believing it would have restored
# the record; it does not.

echo "== a specification accepted record by record (FORMAT-1.0 §4.5, C.3) =="
SP_ROOT=$(scratch_corpus SP)
[[ -d "${SP_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
SP_TMP=$(mktemp -d)
SCRATCH_CORPORA+=("$SP_ROOT" "$SP_TMP")
SP_WAR="$REPO_ROOT/${WAR#./}"
sp_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sp_war() { env -u OPENWARRANT_ACTOR -u OPENWARRANT_CORPUS_ACTS -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$SP_WAR" --root "$SP_ROOT" "$@" </dev/null; }
# sp_acceptance: `record=value` for each acceptance state, sorted, one line.
sp_acceptance() { sp_war --json model 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)
print(" ".join(sorted(s["record"] + "=" + s["value"] for s in v["result"]["states"] if s["kind"] == "acceptance")))' 2>&1; }

cat > "$SP_ROOT/SPEC.md" <<'EOF'
---
format: openwarrant/1.0
profile: spec
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01
version: 0.3.0
summary: Password reset for the account service.
---
# Account service

## Purpose
Users who forget a password regain access without support staff.

## Scope
In: reset by email link. Out: SMS, account recovery without email.

## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once and only within 30
  minutes of issue.

## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
EOF
mkdir -p "$SP_ROOT/docs/tickets"
cat > "$SP_ROOT/docs/tickets/t-5a11e.md" <<'EOF'
---
format: openwarrant/1.0
profile: ticket
alias: t-5a11e
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5a5a11
summary: Send the password reset email.
implements: REQ-reset-1
---
# Reset email

## Checklist
- [ ] Email template (i-5a01)
EOF
# The throwaway key and the register that binds it to a human authorizer.
# Fail-closed: no key, no plant.
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID ssh-keygen -q -t ed25519 -N '' -C plant-throwaway -f "$SP_TMP/key" </dev/null >/dev/null 2>&1 \
    && [[ -s "$SP_TMP/key.pub" ]] \
    || { printf 'PLANT SETUP FAILED: ssh-keygen could not make a throwaway key\n' >&2; exit 9; }
printf 'alice@plant.invalid namespaces="oh.war/response" %s\n' "$(cat "$SP_TMP/key.pub")" > "$SP_ROOT/docs/authority/allowed_signers"
cat > "$SP_ROOT/docs/authority/roles.toml" <<'EOF'
[[assignment]]
actor = "alice"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "alice"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "alice@plant.invalid"
EOF

# --- the request, a signed response, believed ------------------------------------------
SP_REQ=$(sp_war --json spec accept 2>/dev/null)
SP_RESP=$(python3 -c '
import json, sys
r = json.loads(sys.argv[1])["result"]
q = json.dumps
recs = ", ".join("[" + ", ".join(q(x) for x in c) + "]" for c in r["records"])
path = sys.argv[2] + "/docs/authority/responses/" + r["subject"] + ".response.toml"
lines = [
    "schema = " + q("oh.war/accept-response/v1"),
    "subject = " + q(r["subject"]),
    "subject_digest = " + q(r["subject_digest"]),
    "records = [" + recs + "]",
    "accepted_by = " + q("alice"),
    "acting_role = " + q("authorizer"),
    "meaning = " + q("plant: accept every record"),
    "effective_time = " + q("2026-10-14T10:00:00Z"),
]
open(path, "w").write("\n".join(lines) + "\n")
print(path)' "$SP_REQ" "$SP_ROOT" 2>&1)
[[ -f "$SP_RESP" ]] || { printf 'PLANT SETUP FAILED: no response drafted: %s\n' "$SP_RESP" >&2; exit 9; }
( cd "$(dirname "$SP_RESP")" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID ssh-keygen -Y sign -f "$SP_TMP/key" -n oh.war/response "$(basename "$SP_RESP")" </dev/null >/dev/null 2>&1 ) \
    && [[ -s "$SP_RESP.sig" ]] \
    || { printf 'PLANT SETUP FAILED: the throwaway key did not sign\n' >&2; exit 9; }
SP_IN=$(sp_war spec accept --response "$SP_RESP" 2>&1)
SP_ALL=$(sp_acceptance)

# --- one record edited: it alone lapses ---------------------------------------------------
sed -i 's/only within 30$/only within 15/' "$SP_ROOT/SPEC.md"
SP_PART=$(sp_acceptance)
SP_NEXT=$(sp_war --json next 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)
print(" ".join(a["actor"] + ":" + a["command"] for a in v["result"]["actions"] if "spec" in a["command"]))' 2>&1)
SP_IMPACT=$(sp_war impact REQ-reset-1 2>&1)
SP_WANT_ALL="CON-1=accepted REQ-reset-1=accepted S-purpose=accepted S-scope=accepted doc:SPEC.md=accepted"
SP_WANT_PART="CON-1=accepted S-purpose=accepted S-scope=accepted doc:SPEC.md=partially_accepted"
if [[ "$SP_ALL" == "$SP_WANT_ALL" && "$SP_PART" == "$SP_WANT_PART" && "$SP_NEXT" == "human:war sign spec" ]] \
    && grep -q 'spec.accepted' <<<"$SP_IN" \
    && grep -qE 'WARN +impact\.unaccepted .*edited after the act.*t-5a11e' <<<"$SP_IMPACT"; then
    sp_ok "an edited record lapses only itself" "signed accept: accepted; REQ-reset-1 edited: partially_accepted, the rest still accepted; next: $SP_NEXT; impact names t-5a11e"
else
    sp_fail "an edited record lapses only itself" "all '$SP_ALL'; part '$SP_PART'; next '$SP_NEXT'; ingest $(grep -o 'spec\.[a-z-]*\|authority\.[a-z-]*' <<<"$SP_IN" | head -2 | tr '\n' ' '); impact $(grep -o 'impact\.unaccepted.*' <<<"$SP_IMPACT" | head -1)"
fi

# --- the same act over a stale digest is refused, and restores nothing -------------------
SP_ERR=$(sp_war spec accept --response "$SP_RESP" 2>&1); SP_S=$?
SP_AFTER=$(sp_acceptance)
if [[ $SP_S -ne 0 && "$SP_AFTER" == "$SP_WANT_PART" ]] \
    && grep -qE 'ERROR +spec\.stale-digest .*REQ-reset-1 is at sha256:' <<<"$SP_ERR"; then
    sp_ok "an act over a stale digest is refused" "spec.stale-digest names REQ-reset-1's revision now; still partially_accepted"
else
    sp_fail "an act over a stale digest is refused" "exit $SP_S; after '$SP_AFTER': $(head -3 <<<"$SP_ERR" | tr '\n' '|')"
fi
