---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be73-76f7-9aa7-d883cc39d51e
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 4's exit is "registered WARs use KF as institutional authority
while Git may remain Source Holder." OW-WAR-0028 and OW-WAR-0029 delivered §67's
32 typed actions, the §67.1 envelope, optimistic concurrency, idempotency, and
the §12 identity layers.

Nothing has been registered. Every Warrant here carries `enterprise_id = ""`,
which the manifest validator ENFORCES — a non-empty value is refused as
fabricated (§12.4). That refusal has never been tested against a real allocator,
because there has never been one.

The two authorities in the exit criterion are the whole difficulty. KF becomes
institutional authority; Git stays Source Holder. A registration that quietly
takes source authority too would satisfy a careless reading and destroy the
property §91.3 test 21 exists to protect.

## Desired Outcome

At least one Warrant registered through real KF typed actions, holding a
real allocated enterprise identifier, with Git still authoritative for its
source bytes and a round trip proving it.

## Scope

§67 in full, §83, §12, §68 preservation, §91.3 tests 19 and 22, §91.13 tests 91–95.

## Non-goals

- No migration of the whole corpus. One registered Warrant discharges the
  exit; forty is throughput, not proof.
- No clinical or regulated deployment of KF.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-003`, `WAR-SAS-RQ-004`, `WAR-SAS-RQ-005` — Complete; §12 and §83 govern.
- Discharges §98 Phase 4 exit and §99 criteria 12, 13 and 24.
