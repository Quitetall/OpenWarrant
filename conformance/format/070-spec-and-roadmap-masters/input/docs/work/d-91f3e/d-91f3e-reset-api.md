---
format: openwarrant/1.0
profile: design
alias: d-91f3e
uuid: 01a1b788-533f-76ca-8adf-73de6315a6f2
summary: The reset endpoint other repositories call.
type: interface
implements: spec/REQ-reset-1
---
# Reset API

## Contract
POST /reset with an email; 202 whether or not the account exists.
