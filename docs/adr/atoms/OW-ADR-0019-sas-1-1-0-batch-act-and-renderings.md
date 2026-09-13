---
schema: oh.war/atom/v1
adr_uuid: 01a0983e-4a11-7c62-9f3d-6b2e8c5d4071
local_alias: OW-ADR-0019
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0983e-32d7-7473-b6e1-fbef061a1e5f"
---

# ADR OW-0019: SAS 1.1.0 — the batch act, and renderings hold no authority

## Status

Proposed. Accepted when the repository owner signs SAS revision 1.1.0
(§101.3 requires this ADR for an architecture-changing revision, and the
revision is not accepted until that signature exists).

## Context

Signing is per-act today, so clearing a day costs one `ssh-add -c` dialog per
act — twenty-four on 2026-09-12, differing only in the alias each names. The
owner has said twice that this is too much: "Signing a warrant should be
effortless, but something a human does only," and "Nobody wants to open up
another terminal and sign 10 warrants and have to write 20 sentences."

Two remedies exist. Dropping `-c` deletes the dialogs and the control
`docs/THREAT_MODEL.md` entry 1 names with them: the key then signs whatever
asks it, an agent that reached `SSH_AUTH_SOCK` included. Asked directly on
2026-09-12, the owner chose the other: make one signature cover a list they
read.

SAS 1.0.0 cannot say that. §27.2 enumerates human-only acts one at a time,
and §34.4, §38 and §56.1 each bind a signature to a single record. Nothing in
the accepted text describes a signature whose subject is a list of acts. The
same text is silent on a second thing about to be built: a terminal dashboard,
which renders the corpus and must be defined as authority-free before it
exists, not after.

## Decision

## What SAS 1.1.0 adds

**1. The batch act (new §27.6, cited by §27.2).** One signature MAY authorize
many acts when all of the following hold, and the tool SHALL refuse the batch
otherwise:

- the signed subject is a canonical `oh.war/batch/v1` document listing every
  act it covers — act kind, target, and the digest each act is bound to;
- every listed act is one the same signer could have performed individually
  at that moment, under the same role, with no act of a kind the signer is
  not permitted;
- no listed act's bound digest has moved since the batch was drafted. A moved
  digest invalidates the **whole** batch: the batch is re-drafted and
  re-signed, never silently re-scoped;
- the batch is signed by a human in the register, under a declared namespace,
  exactly as a single act is. An agent-kind actor signing a batch is refused
  by the same rule that refuses it a single act;
- each act still writes its own record, and each record cites the batch by
  digest. A reader of one record can find every other act the same signature
  authorized, which is the property that makes one dialog honest.

**2. Attestation of a batch (amends §85's rule).** A batch emits one DSSE
envelope whose subject is the batch document and every record it produced.
The per-act envelopes remain, each naming the batch digest as an additional
subject, so a foreign verifier can check either direction.

**3. A rendering holds no authority (new §76.6).** A rendering is a view of
the corpus — the Pages projection, a terminal dashboard, a printed board. It
MAY issue commands and MUST NOT perform an act: no rendering holds a key,
and a rendering that appears to sign is a defect. Every approval a rendering
offers is the drafting half of a seam whose other half is a human's
signature.

**4. §106 rows.** Three: the batch refusals above; the invalidation rule; and
the rendering rule, each with the conformance test that demonstrates it.

## Why not the alternatives

- **Drop `-c`.** Removes the dialogs by removing the control; an agent that
  reaches the socket then signs silently. Refused on 2026-09-12.
- **A signature per act, accepted as the cost.** Honest, and the owner has
  said twice it is too much. The friction is not the confirmation, it is
  confirming the same decision twenty-four times.
- **A session token ("approve everything for ten minutes").** Time is not a
  subject: the signer would be authorizing acts that did not exist when they
  signed. The batch document exists first, and its digest is what is signed.

## Consequences

- `war sign --batch` becomes sayable (OW-WAR-0072 builds it).
- A dashboard becomes sayable as a rendering (OW-WAR-0073 builds it).
- Every Warrant stays pinned to the revision it was authorized against;
  this revision does not move any contract digest by itself.

## Alternatives rejected

- **Drop `ssh-add -c`.** One keystroke signs everything, and so does anything
  else that reaches the agent socket. Refused by the owner on 2026-09-12.
- **Keep a signature per act.** Honest and unchanged; the owner has rejected
  the cost twice. The friction is not the confirming, it is confirming the
  same decision twenty-four times.
- **A time-boxed session ("approve everything for ten minutes").** Time is not
  a subject: the signer would authorize acts that did not exist when they
  signed. A batch document exists before the signature and its digest is what
  is signed.
- **A tool-held key with a policy engine.** The key would be the tool's, which
  is the failure this whole system exists to prevent.

## Consequences

- `war sign --batch` becomes sayable; OW-WAR-0072 implements it, including the
  refusal that invalidates a whole batch when one bound digest moved.
- A dashboard becomes sayable as a rendering; OW-WAR-0073 implements it under
  §76.6, and a rendering that holds a key is then a defect with a name.
- Warrants authorized against 1.0.0 keep that Basis until an amendment
  re-pins them and a human re-authorizes. This revision moves no contract
  digest by itself.
- Attestations gain a shape for a batch: one envelope over the batch and its
  records, plus each act's own envelope naming the batch digest.
