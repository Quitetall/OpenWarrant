# shellcheck shell=bash
# Authority enforcement: an authority record is believed because a human signed
# it, not because a file says so.
#
# Until these rules existed, the whole test for "this Warrant is authorized" was
# that authorization.toml said `authorized` and named an authorizer — three
# strings an agent can write. A hand-written record naming the owner, whose
# `meaning` read "FORGED BY AN AGENT. No human saw this", passed `war check`
# with zero errors and satisfied §56.1 requirement 1 (2026-09-19, against the
# published 1.0.0-alpha.1). Each plant below removes one thing a real signature
# has and expects the refusal that names it.

SIGNED_WARRANT="OW-WAR-0001"
RESPONSE="docs/authority/responses/$SIGNED_WARRANT.response.toml"

# 1. The signature is gone. The record is untouched and still says `authorized`.
plant "an authorization with no signature" "authority.unsigned" "$SIGNED_WARRANT" 2 \
    "rm -f \"$RESPONSE.sig\"; assert_gone_file \"$RESPONSE.sig\""

# 2. The signature is over other bytes: the response is edited after signing.
plant "a response edited after signing" "authority.signature-invalid" "$SIGNED_WARRANT" 2 \
    "printf '\nplanted = true\n' >> \"$RESPONSE\"; assert_present 'planted = true' \"$RESPONSE\""

# 3. A valid signature moved onto another Warrant's act. The bytes verify; they
#    bind a contract this record does not, which is the whole point of pinning
#    the digest into the response.
plant "a signature replayed onto another Warrant" "authority.signature-invalid" "OW-WAR-0002" 2 \
    "cp \"$RESPONSE\" docs/authority/responses/OW-WAR-0002.response.toml; \
     cp \"$RESPONSE.sig\" docs/authority/responses/OW-WAR-0002.response.toml.sig; \
     assert_present 'OW-WAR-0001' docs/authority/responses/OW-WAR-0002.response.toml"

# 4. The register binds the act to a principal no key in allowed_signers names,
#    so `ssh-keygen -Y verify -I` refuses: a signature by somebody is not a
#    signature by the person the record names.
plant "a principal no key answers for" "authority.signature-invalid" "$SIGNED_WARRANT" 2 \
    "sed -i 's/^ssh_principal = \"brian\"/ssh_principal = \"mallory\"/' docs/authority/roles.toml; \
     assert_present 'mallory' docs/authority/roles.toml"

# 5. §27.2 reserves the act for a human. An agent with a working key is still
#    not an authorizer, and the register is where that is decided.
plant "an agent named as the authorizer" "authority.actor-not-human" "$SIGNED_WARRANT" 2 \
    "sed -i '/^actor = \"Brian Lam\"/,/^ssh_principal/ s/^actor_kind = \"human\"/actor_kind = \"agent\"/' docs/authority/roles.toml; \
     assert_gone 'actor_kind = \"human\"' docs/authority/roles.toml"

# 6. A resolution is the act that says the work is done, and was trusted on
#    content alone for exactly as long as the authorization was.
RESOLVED=$(ls docs/authority/responses/*.resolution.response.toml 2>/dev/null | head -1)
if [[ -n "$RESOLVED" ]]; then
    RESOLVED_ALIAS=$(basename "$RESOLVED" .resolution.response.toml)
    plant "a resolution with no signature" "authority.unsigned" "$RESOLVED_ALIAS" 2 \
        "rm -f \"$RESOLVED.sig\"; assert_gone_file \"$RESOLVED.sig\""
fi

# 7. Positive: the corpus as committed carries signatures that verify, and the
#    check reports them as such. A battery of refusals with no positive proves
#    only that the rule can say no.
SIGNED_COUNT=$("$WAR" check 2>&1 | grep -c 'PASS authority.signed')
if [[ "$SIGNED_COUNT" -gt 0 ]]; then
    printf 'ok    %-34s %s act(s) verify against allowed_signers\n' "signatures verify" "$SIGNED_COUNT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s no act reports a verified signature\n' "signatures verify"
    FAILED=$((FAILED + 1))
fi
