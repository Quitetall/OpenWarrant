# Context: t-0d61d
<!-- oh.war/context/v1 · budget none · depth 3 · preset human (walk core) · selection sha256:880b6cf501b11a4a54398a5ddb73c8c4ec6533219ceb0e2cc820be523edc3958 -->

## Index
[F] t-0d61d · ticket · open · Password reset by email · docs/tickets/t-0d61d.md:1
[F] t-0d61d/S-intent · section · - · Intent · docs/tickets/t-0d61d.md:11
[F] t-0d61d/S-checklist · section · - · Checklist · docs/tickets/t-0d61d.md:14
[F] t-0d61d/i-9c01 · item · open · Reset endpoint · docs/tickets/t-0d61d.md:15
[F] t-0d61d/i-4d11 · item · open · Email template · docs/tickets/t-0d61d.md:16
[F] t-0d61d/i-77be · item · blocked · Docs · docs/tickets/t-0d61d.md:17
[F] t-0d61d/i-5e02 · item · open · Rate limit · docs/tickets/t-0d61d.md:18
[F] t-0d61d/CHK-reset · check · - · reset tests pass · docs/tickets/t-0d61d.md:20
[F] REQ-reset-1 · requirement · - · a reset link works once · SPEC.md:16
[F] CON-1 · constraint · - · no new datastore · SPEC.md:21
[F] OW-WAR-0150/OBL-001 · obligation · - · a consumed link is refused · docs/warrants/OW-WAR-0150.md:20
[F] doc:SPEC.md · spec · - · Account service · SPEC.md:1
[F] OW-WAR-0150 · warrant · draft · Reset links are single use · docs/warrants/OW-WAR-0150.md:1
[F] CSQ-1 · consequence · - · the token table needs a sweeper · SPEC.md:32

## Records
> t-0d61d · ticket · docs/tickets/t-0d61d.md:1 · sha256:1913e182884c
---
format: openwarrant/1.0
profile: ticket
alias: t-0d61d
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10
summary: Password reset by email.
implements: REQ-reset-1
---
# Password reset by email


> t-0d61d/S-intent · section · docs/tickets/t-0d61d.md:11 · sha256:59a4183ac803
<!-- rec t-0d61d/S-intent docs/tickets/t-0d61d.md:11 sha256:59a4183ac803625dd391e5deac0dd1bb39559b9125189a4c3d682d3a347d7173 -->
## Intent
Users reset a password with a link sent by email.

<!-- /rec -->

> t-0d61d/S-checklist · section · docs/tickets/t-0d61d.md:14 · sha256:4d986c82be2d
<!-- rec t-0d61d/S-checklist docs/tickets/t-0d61d.md:14 sha256:4d986c82be2d378127ff23ace01daedceb7100fe472370042df14d5c8894b540 -->
## Checklist
- [ ] Reset endpoint (i-9c01)
- [ ] Email template (i-4d11)
- [ ] Docs (i-77be, after i-4d11)
- [ ] Rate limit (i-5e02)

<!-- /rec -->

> t-0d61d/CHK-reset · check · docs/tickets/t-0d61d.md:20 · sha256:4b51160ff4bc
<!-- rec t-0d61d/CHK-reset docs/tickets/t-0d61d.md:20 sha256:4b51160ff4bc91fc9e2527a9b794582e14c4704659816b085419472848271d45 -->
## CHK-reset · check — reset tests pass
run: cargo test -p accounts reset::
evaluates i-9c01, i-4d11
<!-- /rec -->

> REQ-reset-1 · requirement · SPEC.md:16 · sha256:67903d35987e
<!-- rec REQ-reset-1 SPEC.md:16 sha256:67903d35987e5579735bc1e28ac231f988204e2c56508c2d2065b6389b800977 -->
## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.

<!-- /rec -->

> CON-1 · constraint · SPEC.md:21 · sha256:eee0c7751eac
<!-- rec CON-1 SPEC.md:21 sha256:eee0c7751eacd114eee16fd4633a83274cf3072194cb2a8cc521c18f62b4b7e0 -->
## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1

<!-- /rec -->

> OW-WAR-0150/OBL-001 · obligation · docs/warrants/OW-WAR-0150.md:20 · sha256:fd56c1e72596
<!-- rec OW-WAR-0150/OBL-001 docs/warrants/OW-WAR-0150.md:20 sha256:fd56c1e72596c93f106973891f68e4a34486a2c351e52e0d512861f388d04783 -->
## OBL-001 · obligation — a consumed link is refused
scope: the reset endpoint, one consumed link.
evidence: the second use answers 410.
refusal: a fresh link is accepted.
evaluates REQ-reset-1
<!-- /rec -->

> doc:SPEC.md · spec · SPEC.md:1 · sha256:123d49465d23
---
format: openwarrant/1.0
profile: spec
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01
version: 0.3.0
summary: Password reset for the account service.
---
# Account service


> OW-WAR-0150 · warrant · docs/warrants/OW-WAR-0150.md:1 · sha256:a9af8262ddec
---
format: openwarrant/1.0
profile: delivery
alias: OW-WAR-0150
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e20
summary: Reset links are single use.
---
# Reset links are single use


> CSQ-1 · consequence · SPEC.md:32 · sha256:faa0b685c248
<!-- rec CSQ-1 SPEC.md:32 sha256:faa0b685c24813c48716a6eeb546db7c8793ea4766e8c7fa37c52609e03a3047 -->
## CSQ-1 · consequence — the token table needs a sweeper
<!-- /rec -->

