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
statement: A reset link MUST be accepted at most once and only within 30
  minutes of issue.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.

## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database, which already holds the
  account rows, the session rows and the audit rows, is backed up every hour, is replicated
  to the standby region within a minute, and is the only store the operations team is
  staffed to restore at night, so a second store would double the restore drill.
