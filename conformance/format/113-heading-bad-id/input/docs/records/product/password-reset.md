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

## con-pr1 · constraint — reset links are single use
constrains REQ-pr1
