---
schema: oh.war/records/v1
profile: architecture
---
# Password reset: architecture

The interfaces the reset flow exposes and what the token decision
(DEC-pr1, in 10-records.md) commits the system to. The architecture view
(generated/architecture.md) shows them with the decision.

## IF-pr1 · interface
implements REQ-pr1

`POST /password-reset` with an email address. Always answers 202, whether
or not an account has that address, and emails a link carrying a token.

## IF-pr2 · interface
implements REQ-pr1
depends_on IF-pr1

`POST /password-reset/confirm` with the token and a new password. Answers
204 when the password is changed, and 410 for a token that is expired,
already used or revoked.

## CSQ-pr1 · consequence
part_of DEC-pr1

Every confirm reads the revocation list, so the list is on the reset path
and must be available whenever resets are. An entry can be dropped once
the token it revokes has expired.
