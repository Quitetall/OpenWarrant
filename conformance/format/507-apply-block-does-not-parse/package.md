# Context: t-0d61d
<!-- oh.war/context/v1 · budget none · depth 3 · preset human (walk core) · selection sha256:a1d78b9a1292f8ff7d772c02323071174f7ba2583c971adb467a7ddb7c512769 -->

## Index
[F] t-0d61d · ticket · in_progress · Password reset by email · docs/tickets/t-0d61d.md:1
[F] t-0d61d/S-intent · section · - · Intent · docs/tickets/t-0d61d.md:10
[F] t-0d61d/S-checklist · section · - · Checklist · docs/tickets/t-0d61d.md:13
[F] t-0d61d/i-9c01 · item · done · Reset endpoint · docs/tickets/t-0d61d.md:14
[F] t-0d61d/i-4d11 · item · open · Email template · docs/tickets/t-0d61d.md:15
[F] t-0d61d/CHK-reset · check · - · reset tests pass · docs/tickets/t-0d61d.md:17

## Records
> t-0d61d · ticket · docs/tickets/t-0d61d.md:1 · sha256:2c3818a09517
---
format: openwarrant/1.0
profile: ticket
alias: t-0d61d
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10
summary: Password reset by email.
---
# Password reset by email


> t-0d61d/S-intent · section · docs/tickets/t-0d61d.md:10 · sha256:59a4183ac803
<!-- rec t-0d61d/S-intent docs/tickets/t-0d61d.md:10 sha256:59a4183ac803625dd391e5deac0dd1bb39559b9125189a4c3d682d3a347d7173 -->
## Intent
Users reset a password with a link sent by email.

<!-- /rec -->

> t-0d61d/S-checklist · section · docs/tickets/t-0d61d.md:13 · sha256:a98e4a2d7078
<!-- rec t-0d61d/S-checklist docs/tickets/t-0d61d.md:13 sha256:a98e4a2d707852522d944023cab66a80fc6e8e5d4a83dcc988f4cdf784505508 -->
## Checklist
- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12
- [ ] Email template (i-4d11)

<!-- /rec -->

> t-0d61d/CHK-reset · check · docs/tickets/t-0d61d.md:17 · sha256:4b51160ff4bc
<!-- rec t-0d61d/CHK-reset docs/tickets/t-0d61d.md:17 sha256:4b51160ff4bc91fc9e2527a9b794582e14c4704659816b085419472848271d45 -->
## CHK-reset · check — reset tests pass
run: cargo test -p accounts reset::
evaluates i-9c01, i-4d11
Not a field.
<!-- /rec -->

