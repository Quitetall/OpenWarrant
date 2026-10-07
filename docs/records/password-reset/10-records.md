---
schema: oh.war/records/v1
profile: delivery
---
# Password reset

The worked example of the semantic model (§5.1): one outcome, the
requirement that serves it, a constraint on that requirement, and the
decision taken between two options. Each `## <ID> · <type>` opens a record;
a line `<kind> <ID>` inside one is a relation. Warrants, tickets and the
roadmap name these records by id; `war impact REQ-pr1` lists what a change
to one affects. docs/TYPES.md explains the format.

## OUT-pr1 · outcome

A user who forgot their password regains access without support.

## REQ-pr1 · requirement
implements OUT-pr1

A reset token expires 15 minutes after issue.

## CON-pr1 · constraint
constrains REQ-pr1

Tokens are single-use and stored hashed.

## DEC-pr1 · decision
selected_over OPT-pr1, OPT-pr2
constrains REQ-pr1

Signed, stateless tokens with a server-side revocation list: expiry is
checked from the token itself, and a used or revoked token is refused by
the list.

## OPT-pr1 · option

Random opaque tokens stored server-side, looked up on every use.

## OPT-pr2 · option

Signed tokens with no server-side state at all; a used token cannot be
refused before it expires.
