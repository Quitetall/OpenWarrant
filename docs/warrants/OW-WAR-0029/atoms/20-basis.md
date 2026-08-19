---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-707d-b64a-27ffb62e8931
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §12 (identity layers, enterprise identifier, federation record, offline creation, stable references), §83.

## Prerequisites

OW-WAR-0028 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** §12 fully specifies the layers and this repository already honours the local half.
- **Accepted residual risk.** The OpenHuman Identifier Registry does not exist yet (§101.5); until it does, allocation has no authority to call.

## Constraints and Invariants

- **Registration does not transfer Source Holder** (RQ-004, §91.3 test 21).
- **Offline creation keeps working** (§12.6, RQ-070). Federation is additive.
- **The UUID never changes** (§91.3 test 17) when an enterprise ID is allocated.
- **An enterprise ID is never minted locally** (§91.3 test 20) — the existing refusal stays.
