---
format: openwarrant/1.0
profile: runbook
owner: ops
summary: Restart the account service.
---
# Restart the account service

## Steps
## STEP-1 · step — restart the unit
command: systemctl restart accounts
timeout: 30
severity: high

Wait for the health check to pass.

## W-1 · widget — a widget the profile does not name
