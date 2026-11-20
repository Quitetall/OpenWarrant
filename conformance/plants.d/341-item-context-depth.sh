# shellcheck shell=bash
# `war context <item>` (FORMAT-1.0 10.3 step 1, 10.4 agent.min): an item of a
# ticket that has no Intent section gets the ticket's record at distance 1
# instead. The ticket's REQ must then bring its constraint within the
# preset's depth 2, as it does for the ticket itself. Each claim has its
# refusal.
# - the item's context holds the ticket's REQ and that REQ's constraint.
# - refusal: `--depth 1` stops at the REQ, no constraint.
# - refusal: an explicit budget too small for the selection is still
#   refused (context.over-budget), not trimmed silently.
# - control: a ticket WITH an Intent keeps its item context unchanged: the
#   Intent, and no ticket record.

echo "== an item's context reaches its REQ's constraint =="
CD_TMP=$(mktemp -d)
CD_WAR="$REPO_ROOT/${WAR#./}"
cd_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cd_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
cd_war() {
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID \
        -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
        OPENWARRANT_ACTOR=plant timeout 120 "$CD_WAR" --root "$CD_TMP/repo" "$@" </dev/null
}
mkdir -p "$CD_TMP/repo/docs/tickets"
git -C "$CD_TMP/repo" init -q .
cd_war init >/dev/null 2>&1
cat > "$CD_TMP/repo/SPEC.md" <<'SPEC'
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
In: reset by email link. Out: SMS.

## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.

## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1
SPEC
ticket() { # alias uuid-tail intent-block
    printf -- '---\nformat: openwarrant/1.0\nprofile: ticket\nalias: %s\nuuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e%s\nsummary: Password reset by email.\nimplements: REQ-reset-1\n---\n# Password reset by email\n\n%s## Checklist\n- [ ] Email template (i-4d11)\n' \
        "$1" "$2" "$3" > "$CD_TMP/repo/docs/tickets/$1.md"
}
ticket t-0d61d 10 ''
ticket t-0d62e 11 $'## Intent\nUsers reset a password with a link sent by email.\n\n'
git -C "$CD_TMP/repo" add -A >/dev/null 2>&1
git -C "$CD_TMP/repo" -c user.email=plant@invalid -c user.name=plant commit -qm fixtures >/dev/null 2>&1

CD_OUT=$(cd_war view context t-0d61d/i-4d11 2>&1)
if grep -q '^\[F\] REQ-reset-1 ' <<<"$CD_OUT" && grep -q '^\[F\] CON-1 ' <<<"$CD_OUT"; then
    cd_ok "item reaches REQ and constraint" "REQ-reset-1 and CON-1 are in the default (agent.min) context"
else
    cd_fail "item reaches REQ and constraint" "$(grep '^\[F\]' <<<"$CD_OUT" | tr '\n' '|' | head -c 400)"
fi
CD_ONE=$(cd_war view context t-0d61d/i-4d11 --depth 1 2>&1)
if grep -q '^\[F\] REQ-reset-1 ' <<<"$CD_ONE" && ! grep -q '^\[F\] CON-1 ' <<<"$CD_ONE"; then
    cd_ok "depth 1 still stops short" "REQ-reset-1 without CON-1"
else
    cd_fail "depth 1 still stops short" "$(grep '^\[F\]' <<<"$CD_ONE" | tr '\n' '|' | head -c 400)"
fi
cd_war view context t-0d61d/i-4d11 --budget 200 >"$CD_TMP/o" 2>&1; CD_RC=$?
if [[ $CD_RC -ne 0 ]] && grep -q 'context.over-budget' "$CD_TMP/o"; then
    cd_ok "an explicit small budget is refused" "context.over-budget, exit $CD_RC"
else
    cd_fail "an explicit small budget is refused" "exit $CD_RC: $(head -c 200 "$CD_TMP/o")"
fi
CD_INT=$(cd_war view context t-0d62e/i-4d11 2>&1)
if grep -q '^\[F\] t-0d62e/S-intent ' <<<"$CD_INT" && ! grep -q '^\[F\] t-0d62e · ticket' <<<"$CD_INT" \
    && ! grep -q '^\[F\] REQ-reset-1 ' <<<"$CD_INT"; then
    cd_ok "a ticket with an Intent is unchanged" "the Intent, no ticket record, no REQ"
else
    cd_fail "a ticket with an Intent is unchanged" "$(grep '^\[F\]' <<<"$CD_INT" | tr '\n' '|' | head -c 400)"
fi
[[ -n "$CD_TMP" && "$CD_TMP" == */tmp.* ]] && /usr/bin/rm -rf "$CD_TMP"
unset CD_TMP CD_WAR CD_OUT CD_ONE CD_INT CD_RC
unset -f cd_ok cd_fail cd_war ticket
