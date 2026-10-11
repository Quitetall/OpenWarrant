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

## TEST-1 · test — the expiry is checked
evaluates REQ-pr1@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
