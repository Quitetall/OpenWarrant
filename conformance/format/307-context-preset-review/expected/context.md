# Context: OW-WAR-0150
<!-- oh.war/context/v1 · budget none · depth 2 · preset review (walk review) · selection sha256:5149cacf1e599d4f6e73a51c09645585fe9b2015a4a479aeb8ca139c973cc2e2 · journal sha256:4f53cda18c2b · at 2026-10-14T10:00:00Z -->

## Index
[F] OW-WAR-0150 · warrant · open · Reset links are single use · docs/warrants/OW-WAR-0150.md:1
[F] OW-WAR-0150/S-intent · section · - · Intent · docs/warrants/OW-WAR-0150.md:10
[F] OW-WAR-0150/S-basis · section · - · Basis · docs/warrants/OW-WAR-0150.md:13
[F] OW-WAR-0150/S-work-order · section · - · Work order · docs/warrants/OW-WAR-0150.md:16
[F] OW-WAR-0150/S-assurance · section · - · Assurance · docs/warrants/OW-WAR-0150.md:19
[F] OW-WAR-0150/OBL-001 · obligation · - · a consumed link is refused · docs/warrants/OW-WAR-0150.md:20
[F] REQ-reset-1 · requirement · - · a reset link works once · SPEC.md:16

## Records
> OW-WAR-0150 · warrant · docs/warrants/OW-WAR-0150.md:1 · sha256:a9af8262ddec
---
format: openwarrant/1.0
profile: delivery
alias: OW-WAR-0150
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e20
summary: Reset links are single use.
---
# Reset links are single use


> OW-WAR-0150/S-intent · section · docs/warrants/OW-WAR-0150.md:10 · sha256:566f2147a0b0
## Intent
A consumed reset link is refused.


> OW-WAR-0150/S-basis · section · docs/warrants/OW-WAR-0150.md:13 · sha256:68ed4ff9fba3
## Basis
REQ-reset-1 of the account service specification.


> OW-WAR-0150/S-work-order · section · docs/warrants/OW-WAR-0150.md:16 · sha256:f3ddf3521fb1
## Work order
Deliverables: the token check. Frozen: the mailer.


> OW-WAR-0150/S-assurance · section · docs/warrants/OW-WAR-0150.md:19 · sha256:20089f674264
## Assurance

> OW-WAR-0150/OBL-001 · obligation · docs/warrants/OW-WAR-0150.md:20 · sha256:fd56c1e72596
## OBL-001 · obligation — a consumed link is refused
scope: the reset endpoint, one consumed link.
evidence: the second use answers 410.
refusal: a fresh link is accepted.
evaluates REQ-reset-1

> REQ-reset-1 · requirement · SPEC.md:16 · sha256:67903d35987e
## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.


