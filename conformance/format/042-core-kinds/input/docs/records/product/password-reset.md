---
format: openwarrant/1.0
profile: prd
summary: What password reset must do.
---
# Password reset

## OUT-pr1 · outcome — users who forget a password regain access

A user locked out of an account can get back in without support.

## REQ-pr1 · requirement — a user can reset a password by email
implements OUT-pr1

The reset link expires after one hour.

## CON-pr1 · constraint — reset links are single use
constrains REQ-pr1

## DEC-1 · decision — email, not SMS
supports OUT-pr1
refutes CON-pr1
trades_off_against REQ-pr1
causes OUT-pr1
qualifies REQ-pr1
supersedes CON-pr1
depends_on REQ-pr1
part_of OUT-pr1
evaluates REQ-pr1
