# Context: t-0d61d
<!-- oh.war/context/v1 · budget 16000 · depth 2 · preset agent.min (walk agent.min) · selection sha256:6785c48c945f57558bb648a7da43142055c1e883caf112c2114b729841b24fea -->

## Index
[F] t-0d61d · ticket · open · Password reset by email · docs/tickets/t-0d61d.md:1
[F] t-0d61d/S-intent · section · - · Intent · docs/tickets/t-0d61d.md:12
[F] t-0d61d/S-checklist · section · - · Checklist · docs/tickets/t-0d61d.md:15
[F] t-0d61d/i-4d11 · item · open · Email template · docs/tickets/t-0d61d.md:16
[F] t-0d61d/i-5e02 · item · open · Rate limit · docs/tickets/t-0d61d.md:17
[F] REQ-reset-1 · requirement · - · a reset link works once · SPEC.md:16
[F] OW-WAR-0150/S-intent · section · - · Intent · docs/warrants/OW-WAR-0150.md:10
[F] CON-1 · constraint · - · no new datastore · SPEC.md:21
[F] DEC-1 · decision · - · links are single-use tokens · SPEC.md:25
[F] OW-WAR-0150/OBL-001 · obligation · - · a consumed link is refused · docs/warrants/OW-WAR-0150.md:20

## Records
> t-0d61d · ticket · docs/tickets/t-0d61d.md:1 · sha256:9e951e2a9141
---
format: openwarrant/1.0
profile: ticket
alias: t-0d61d
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10
summary: Password reset by email.
implements: REQ-reset-1
part_of: OW-WAR-0150
---
# Password reset by email


> t-0d61d/S-intent · section · docs/tickets/t-0d61d.md:12 · sha256:59a4183ac803
## Intent
Users reset a password with a link sent by email.


> t-0d61d/S-checklist · section · docs/tickets/t-0d61d.md:15 · sha256:14fab3a5a88b
## Checklist
- [ ] Email template (i-4d11)
- [ ] Rate limit (i-5e02)

> REQ-reset-1 · requirement · SPEC.md:16 · sha256:67903d35987e
## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
verify: reuse a consumed link; expect refusal.


> OW-WAR-0150/S-intent · section · docs/warrants/OW-WAR-0150.md:10 · sha256:566f2147a0b0
## Intent
A consumed reset link is refused.


> CON-1 · constraint · SPEC.md:21 · sha256:eee0c7751eac
## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1


> DEC-1 · decision · SPEC.md:25 · sha256:4fce08b1c775
## DEC-1 · decision — links are single-use tokens
selected_over OPT-1
causes CSQ-1
constrains REQ-reset-1


> OW-WAR-0150/OBL-001 · obligation · docs/warrants/OW-WAR-0150.md:20 · sha256:fd56c1e72596
## OBL-001 · obligation — a consumed link is refused
scope: the reset endpoint, one consumed link.
evidence: the second use answers 410.
refusal: a fresh link is accepted.
evaluates REQ-reset-1

