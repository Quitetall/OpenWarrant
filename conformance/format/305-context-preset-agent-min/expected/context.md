# Context: SPEC.md
<!-- oh.war/context/v1 · budget 4000 · depth 1 · preset agent.min (walk agent.min) · selection sha256:628163b0526e037cbda7d02c403555cdd23d7a0298e7996bb9bd5b4ace849509 -->

## Index
[F] S-purpose · section · - · Purpose · SPEC.md:10
[F] S-scope · section · - · Scope · SPEC.md:13
[F] REQ-reset-1 · requirement · - · a reset link works once · SPEC.md:16
[F] CON-1 · constraint · - · no new datastore · SPEC.md:21
[F] DEC-1 · decision · - · links are single-use tokens · SPEC.md:25
[F] doc:SPEC.md · spec · - · Account service · SPEC.md:1
[F] OW-WAR-0150/OBL-001 · obligation · - · a consumed link is refused · docs/warrants/OW-WAR-0150.md:20

## Records
> S-purpose · section · SPEC.md:10 · sha256:b1d6782792c2
## Purpose
Users who forget a password regain access without support staff.


> S-scope · section · SPEC.md:13 · sha256:d275a57991a0
## Scope
In: reset by email link. Out: SMS, account recovery without email.


> REQ-reset-1 · requirement · SPEC.md:16 · sha256:67903d35987e
## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
verify: reuse a consumed link; expect refusal.


> CON-1 · constraint · SPEC.md:21 · sha256:eee0c7751eac
## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1


> DEC-1 · decision · SPEC.md:25 · sha256:d8feba6fc56d
## DEC-1 · decision — links are single-use tokens
selected_over OPT-1
causes CSQ-1
implements REQ-reset-1


> doc:SPEC.md · spec · SPEC.md:1 · sha256:123d49465d23
---
format: openwarrant/1.0
profile: spec
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01
version: 0.3.0
summary: Password reset for the account service.
---
# Account service


> OW-WAR-0150/OBL-001 · obligation · docs/warrants/OW-WAR-0150.md:20 · sha256:fd56c1e72596
## OBL-001 · obligation — a consumed link is refused
scope: the reset endpoint, one consumed link.
evidence: the second use answers 410.
refusal: a fresh link is accepted.
evaluates REQ-reset-1

