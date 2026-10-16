# Context: t-0d61d
<!-- oh.war/context/v1 · budget none · depth 1 · preset resume (walk core) · selection sha256:41b368a369299cf13928ab87fb29869b14565f37d4bade1420bb81cc202e78d2 · journal sha256:93f52f8e4107 · at 2026-10-14T10:00:00Z -->

## Index
[F] t-0d61d · ticket · in_progress · Password reset by email · docs/tickets/t-0d61d.md:1
[F] t-0d61d/i-9c01 · item · done · Reset endpoint · docs/tickets/t-0d61d.md:15
[F] t-0d61d/i-4d11 · item · claimed · Email template · docs/tickets/t-0d61d.md:16
[F] t-0d61d/i-77be · item · blocked · Docs · docs/tickets/t-0d61d.md:17
[F] t-0d61d/i-5e02 · item · open · Rate limit · docs/tickets/t-0d61d.md:18
[F] REQ-reset-1 · requirement · - · a reset link works once · SPEC.md:16
[F] t-0d61d/CHK-reset · check · - · reset tests pass · docs/tickets/t-0d61d.md:20

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


> t-0d61d/i-9c01 · item · docs/tickets/t-0d61d.md:15 · sha256:865e42d2cc75
state done · by claude · 2026-10-12 · claimed
- [x] Reset endpoint (i-9c01) — done by claude, 2026-10-12

> t-0d61d/i-4d11 · item · docs/tickets/t-0d61d.md:16 · sha256:739e9dd2426a
state claimed · holder bob · lease 2026-10-14T10:20:00Z
- [ ] Email template (i-4d11)

> t-0d61d/i-77be · item · docs/tickets/t-0d61d.md:17 · sha256:6c4fed404860
state blocked
- [ ] Docs (i-77be, after i-4d11)

> t-0d61d/i-5e02 · item · docs/tickets/t-0d61d.md:18 · sha256:e772724ccc51
state open
- [ ] Rate limit (i-5e02)

> REQ-reset-1 · requirement · SPEC.md:16 · sha256:67903d35987e
## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.


> t-0d61d/CHK-reset · check · docs/tickets/t-0d61d.md:20 · sha256:4b51160ff4bc
## CHK-reset · check — reset tests pass
run: cargo test -p accounts reset::
evaluates i-9c01, i-4d11

## Next
- t-0d61d/i-5e02 · Rate limit

