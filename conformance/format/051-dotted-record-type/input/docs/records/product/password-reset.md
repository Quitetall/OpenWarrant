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

## X-1 · com.example.risk — reset email lands in spam
constrains OUT-pr1
score: high

Spam filters may hold the mail.
