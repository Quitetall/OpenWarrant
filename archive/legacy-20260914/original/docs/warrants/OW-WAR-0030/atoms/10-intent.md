---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b7-7767-970e-1e270c168858
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`WAR.json` carries sources and relations. §68.2 requires an export to include contract, sources, receipts, assurance, and resolution — most of which do not exist yet — and §68.3 requires export→import→export to preserve semantic identity. A minimal round trip is already tested; the full one is not, because there is nothing to round trip.

## Desired Outcome

One file carries a complete Warrant: contract, sources, receipts, assurance case, and resolution. Export→import→export is byte-stable, and superseded, disputed, and annulled records remain available (RQ-084).

## Scope

The one-file export (§68.1), its contents (§68.2), and round-trip preservation (§68.3).

## Non-goals

- No cross-version migration; §69's additive evolution covers forward compatibility and breaking changes are a protocol decision.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-082` — Complete; governing section in Basis.
- `WAR-SAS-RQ-083` — Complete; governing section in Basis.
- `WAR-SAS-RQ-084` — Complete; governing section in Basis.
