---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-72ba-87b3-c1bd1aec86a8
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

A parsed manifest is a value in one process's memory. Nothing yet gives a
Warrant a canonical form that two hosts agree on byte for byte, and without that
there is no digest, no contract identity, and no export that another system can
verify.

`openwarrant-compiler` currently declares fifteen digest domains and can hash
bytes. It cannot produce the bytes.

## Desired Outcome

A valid Compilation Basis lowers to a canonical WAR IR that serialises to
RFC 8785 canonical JSON, identically on any host, and digests computed over it
are domain-separated so that identical JSON in different semantic domains cannot
collide.

## Scope

The IR shape of §63, the `format_basis` pin of §64, RFC 8785 canonicalization,
and the domain-separated preimage construction of §65.2.

## Non-goals

- No rendering of the human Markdown parent. OW-WAR-0004.
- No `contract` immutability machinery, no attempts, no assurance case. Those IR
  sections exist in the shape but are not populated in Phase 1.
- No Knowledge Fabric export.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-014` — full WAR Markdown and canonical JSON compile from one
  Basis. Partial: the JSON half only; the Markdown half is OW-WAR-0004.
- `WAR-SAS-RQ-080` — the canonical portable WAR is RFC 8785 JSON. Complete.
- `WAR-SAS-RQ-081` — cross-system digests use explicit algorithms and domains.
  Complete.
