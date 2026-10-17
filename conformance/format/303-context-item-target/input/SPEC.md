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
In: reset by email link. Out: SMS, account recovery without email.

## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.

## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1

## DEC-1 · decision — links are single-use tokens
selected_over OPT-1
causes CSQ-1
implements REQ-reset-1

## OPT-1 · option — signed expiring links without state

## CSQ-1 · consequence — the token table needs a sweeper
