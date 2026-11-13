---
format: openwarrant/1.0
profile: design
alias: d-5e21b
uuid: 01a1b788-533f-76ca-8adf-73dac854ecc2
summary: How the read cache is invalidated.
kind: document
type: design
---
# Cache design

## Approach
Entries are keyed by the content digest, so an edit is a miss.
