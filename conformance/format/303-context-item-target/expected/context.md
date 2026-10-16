# Context: t-0d61d/i-4d11
<!-- oh.war/context/v1 · budget 16000 · depth 3 · preset agent (walk core) · selection sha256:2046fbd58933c50f0cf602cd890538c659561ee5addccf278f87da67f3a39b9d -->

## Index
[F] t-0d61d/i-4d11 · item · open · Email template · docs/tickets/t-0d61d.md:16
[F] t-0d61d · ticket · in_progress · Password reset by email · docs/tickets/t-0d61d.md:1
[F] t-0d61d/S-intent · section · - · Intent · docs/tickets/t-0d61d.md:11
[F] t-0d61d/CHK-reset · check · - · reset tests pass · docs/tickets/t-0d61d.md:20
[F] REQ-reset-1 · requirement · - · a reset link works once · SPEC.md:16
[F] CON-1 · constraint · - · no new datastore · SPEC.md:21
[F] OW-WAR-0150/OBL-001 · obligation · - · a consumed link is refused · docs/warrants/OW-WAR-0150.md:20
[F] doc:SPEC.md · spec · - · Account service · SPEC.md:1

## Records
> t-0d61d/i-4d11 · item · docs/tickets/t-0d61d.md:16 · sha256:739e9dd2426a
- [ ] Email template (i-4d11)

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
## Intent
Users reset a password with a link sent by email.


> t-0d61d/CHK-reset · check · docs/tickets/t-0d61d.md:20 · sha256:4b51160ff4bc
## CHK-reset · check — reset tests pass
run: cargo test -p accounts reset::
evaluates i-9c01, i-4d11

> REQ-reset-1 · requirement · SPEC.md:16 · sha256:67903d35987e
## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.


> CON-1 · constraint · SPEC.md:21 · sha256:eee0c7751eac
## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1


> OW-WAR-0150/OBL-001 · obligation · docs/warrants/OW-WAR-0150.md:20 · sha256:fd56c1e72596
## OBL-001 · obligation — a consumed link is refused
scope: the reset endpoint, one consumed link.
evidence: the second use answers 410.
refusal: a fresh link is accepted.
evaluates REQ-reset-1

> doc:SPEC.md · spec · SPEC.md:1 · sha256:123d49465d23
---
format: openwarrant/1.0
profile: spec
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01
version: 0.3.0
summary: Password reset for the account service.
---
# Account service


