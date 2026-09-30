---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32d9-76e0-8417-2d69773249ca
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war sign` drafts one response document per act, and the human confirms each
one through `ssh-add -c`. Twenty-four acts is twenty-four dialogs that differ
only in the alias they name. SAS 1.1.0 §27.6 (OW-WAR-0071) makes a different
shape sayable — one signature over a canonical list — and nothing implements
it. Until something does, the queue that `war console` renders in one screen
still costs a dialog a row, which is the friction the owner named.

## Desired Outcome

`war sign --batch` drafts an `oh.war/batch/v1` document listing every checked
act with the digest each is bound to, the human signs that one document, and
the ingest writes each act's own record citing the batch by digest. A moved
digest refuses the whole batch by name. An agent-kind signer is refused by the
same rule that refuses it a single act. Each record remains exactly the record
it would have been, so every existing reader — `war check`, the projections,
the attestation verifier — needs no new case.

## Non-Goals

- Any relaxation of `ssh-add -c`: this exists so the owner need not drop it.
- Multi-signer batches, or a batch spanning two roles.
- Signing anything the signer could not have signed individually at that
  moment. A batch is an economy of dialogs, never of authority.
