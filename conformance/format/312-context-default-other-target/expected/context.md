# Context: REQ-reset-1
<!-- oh.war/context/v1 · budget 16000 · depth 3 · preset agent (walk core) · selection sha256:18e7fe4307b486fbcde3e7657b20c675dd637dd829b7461a515afedabe953c37 -->

## Index
[F] REQ-reset-1 · requirement · - · a reset link works once · SPEC.md:16
[F] CON-1 · constraint · - · no new datastore · SPEC.md:21
[F] DEC-1 · decision · - · links are single-use tokens · SPEC.md:25
[F] doc:SPEC.md · spec · - · Account service · SPEC.md:1
[F] OW-WAR-0150/OBL-001 · obligation · - · a consumed link is refused · docs/warrants/OW-WAR-0150.md:20
[F] OPT-1 · option · - · signed expiring links without state · SPEC.md:31
[F] CSQ-1 · consequence · - · the token table needs a sweeper · SPEC.md:33
[F] OW-WAR-0150 · warrant · open · Reset links are single use · docs/warrants/OW-WAR-0150.md:1

## Records
> REQ-reset-1 · requirement · SPEC.md:16 · sha256:67903d35987e
## REQ-reset-1 · requirement — a reset link works once
statement: A reset link MUST be accepted at most once.
rationale: a reused link is a takeover path.
verify: reuse a consumed link; expect refusal.


> CON-1 · constraint · SPEC.md:21 · sha256:eee0c7751eac
## CON-1 · constraint — no new datastore
statement: Reset state MUST live in the existing accounts database.
constrains REQ-reset-1


> DEC-1 · decision · SPEC.md:25 · sha256:4fce08b1c775
## DEC-1 · decision — links are single-use tokens
selected_over OPT-1
causes CSQ-1
constrains REQ-reset-1
rationale: a token row is cheaper than a signing key to rotate.


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

> OPT-1 · option · SPEC.md:31 · sha256:37d5f08ee162
## OPT-1 · option — signed expiring links without state


> CSQ-1 · consequence · SPEC.md:33 · sha256:faa0b685c248
## CSQ-1 · consequence — the token table needs a sweeper

> OW-WAR-0150 · warrant · docs/warrants/OW-WAR-0150.md:1 · sha256:a9af8262ddec
---
format: openwarrant/1.0
profile: delivery
alias: OW-WAR-0150
uuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e20
summary: Reset links are single use.
---
# Reset links are single use


