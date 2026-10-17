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
retries: 3

Wait for the health check to pass.
