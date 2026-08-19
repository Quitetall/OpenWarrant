---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd0-d818-7d1b-969f-79f172735b78
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Every compilation is contract revision 1. The renderer has a constant saying so.
There is no way to author a second revision, no immutability guarantee on the
first, and nothing preventing progress notes from silently amending what was
authorized — which §28.7 and Law 6 both forbid.

The children in this repository already pin a parent `contract_digest`. That
digest is computed but never frozen: if OW-WAR-0001's contract changed, the
children would report a mismatch, which is right — but nothing stops the change.

## Desired Outcome

A contract revision is an immutable authorized snapshot. Amendments create a new revision rather than editing one. Prior attempts keep the basis they ran under.

## Scope

Draft revisions, proposal snapshots, authorization, contract digests, revision ancestry, and the prohibition on in-place amendment (§28); contract content (§29).

## Non-goals

- No KF-side authorization; who may authorize is OW-WAR-0028. This delivers the
  revision MECHANICS with local authorization as a placeholder that is explicitly
  labelled as such.
- No amendment classification; that is OW-WAR-0010.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-030` — authorized contract revisions are immutable. Complete.
- `WAR-SAS-RQ-031` — progress cannot amend the contract. Complete.
- `WAR-SAS-RQ-033` — material amendment creates a new revision. Complete.
- `WAR-SAS-RQ-034` — prior attempts retain their original contract basis. Complete.
