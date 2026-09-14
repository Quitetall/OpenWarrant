# shellcheck shell=bash
# Known defects fixed on the unpinned seams (slice B2, partial): a malformed
# effective_time refused by authorize and resolve; the performer configured in
# openwarrant.toml; and multi-signer stays possible (slice B6): two eligible
# signers is a question, a duplicate principal is a refusal.

DEF_TMP=$(mktemp -d)
AUTH_DIGEST="$("$WAR" authorize OW-WAR-0010 2>/dev/null | grep '^contract_digest' | cut -d'"' -f2)"
[[ -n "$AUTH_DIGEST" ]] || { echo "could not read OW-WAR-0010's contract digest" >&2; exit 9; }

plant_cmd "authorize refuses effective_time = soon" "authorize.effective-time" "RFC 3339" 2 \
    "printf 'schema = \"oh.war/authorization-response/v1\"\nwarrant = \"OW-WAR-0010\"\ncontract_digest = \"%s\"\nauthorizer = \"Brian Lam\"\nacting_role = \"authorizer\"\nmeaning = \"x\"\neffective_time = \"soon\"\nindependence = \"separate_role\"\n' \"$AUTH_DIGEST\" > \"$DEF_TMP/soon.toml\"" \
    authorize OW-WAR-0010 --response "$DEF_TMP/soon.toml"

plant_cmd "resolve refuses effective_time = soon" "resolution.effective-time" "RFC 3339" 2 \
    "printf 'schema = \"oh.war/resolution-response/v1\"\nwarrant = \"OW-WAR-0063\"\ncontract_digest = \"%s\"\nresolved_by = \"Brian Lam\"\nacting_role = \"resolver\"\ncommon_outcome = \"satisfied\"\nprofile_outcome = \"delivered\"\nmeaning = \"x\"\neffective_time = \"soon\"\n' \"$AUTH_DIGEST\" > \"$DEF_TMP/rsoon.toml\"" \
    resolve OW-WAR-0063 --response "$DEF_TMP/rsoon.toml"

# The performer is configured, not flagged: naming the owner as performer
# makes the owner's own authorization a self-authorization (§27.2).
plant_cmd "a configured performer cannot authorize itself" "authorize.not-permitted" "Brian Lam" 2 \
    "sed -i 's/^namespace = \"OW\"$/namespace = \"OW\"\nperformer = \"Brian Lam\"/' openwarrant.toml; assert_present 'performer = \"Brian Lam\"' openwarrant.toml; \
     printf 'schema = \"oh.war/authorization-response/v1\"\nwarrant = \"OW-WAR-0010\"\ncontract_digest = \"%s\"\nauthorizer = \"Brian Lam\"\nacting_role = \"authorizer\"\nmeaning = \"x\"\neffective_time = \"2026-09-02T00:00:00Z\"\nindependence = \"separate_role\"\n' \"$AUTH_DIGEST\" > \"$DEF_TMP/self.toml\"" \
    authorize OW-WAR-0010 --response "$DEF_TMP/self.toml"

# The two signing plants below need a Warrant awaiting a signature. Which real
# Warrants are unsigned changes every time the owner signs one, so they make
# their own and remove it at the end of the file.
DEF_ALIAS=$(scratch_warrant "the signing refusals")

# Two eligible signers: the tool asks, it does not pick.
plant_cmd "two eligible signers is a question" "sign.who" "--as" 2 \
    "printf '\n[[assignment]]\nactor = \"Second Human\"\nactor_kind = \"human\"\nroles = [\"authorizer\", \"resolver\", \"risk_acceptor\"]\nassigned_by = \"Brian Lam\"\neffective_time = \"2026-01-01T00:00:00Z\"\nssh_principal = \"second\"\n' >> docs/authority/roles.toml; assert_present 'Second Human' docs/authority/roles.toml" \
    sign "$DEF_ALIAS" --ssh-sign

# A principal listed twice in allowed_signers: refused before any signing.
plant_cmd "a duplicate principal is refused" "sign.ssh-refused" "exactly one" 2 \
    "grep '^brian ' docs/authority/allowed_signers | head -1 >> docs/authority/allowed_signers; assert_present 'brian' docs/authority/allowed_signers" \
    sign "$DEF_ALIAS" --ssh-sign --as "Brian Lam"

rm -rf "$DEF_TMP"
scratch_warrant_gone "$DEF_ALIAS"
