---
format: openwarrant/1.0
profile: spec
alias: spec
uuid: 01a1b788-533f-76ca-8adf-73dc22f4ad00
summary: Password reset for the account service.
version: 0.1.0
type: spec
---
# Account service

## Purpose
Users who forget a password regain access without support staff.

## Scope
In: reset by email link. Out: SMS.

## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
