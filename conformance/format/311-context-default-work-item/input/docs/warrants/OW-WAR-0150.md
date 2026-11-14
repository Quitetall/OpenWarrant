---
format: openwarrant/1.0
profile: delivery
alias: OW-WAR-0150
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e20
summary: Reset links are single use.
---
# Reset links are single use

## Intent
A consumed reset link is refused.

## Basis
REQ-reset-1 of the account service specification.

## Work order
Deliverables: the token check. Frozen: the mailer.

## Assurance
## OBL-001 · obligation — a consumed link is refused
scope: the reset endpoint, one consumed link.
evidence: the second use answers 410.
refusal: a fresh link is accepted.
evaluates REQ-reset-1
