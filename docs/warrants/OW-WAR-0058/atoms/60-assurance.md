---
schema: oh.war/atom/v1
warrant_uuid: 01a06069-882d-7103-a07a-8e2d5c23bd12
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an accepted revision is immutable
- **scope:** §101.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a unit test accepts a proposal and shows a second acceptance
  is refused as `Immutable`; no method on the type mutates an accepted
  revision.

### OBL-002 — an agent cannot accept, and an architecture change needs an ADR
- **scope:** §27.2, §101.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a unit test refuses an agent acceptance; a plant ingests a
  response naming `claude` and is refused; a unit test refuses an
  architecture-changing acceptance with no `adr_ref`.

### OBL-003 — the document's bytes are held to the record
- **scope:** §101.6.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant edits one byte of the SAS and `war check` refuses it
  as `sas.digest-drift`, naming the recorded and actual digests.

### OBL-004 — a §106 id cannot be removed
- **scope:** §34.1, §34.4 step 4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant runs `war sas diff` against a candidate with one row
  deleted and is refused, naming the id; an added or retitled row is
  reported and not refused.

### OBL-005 — every Warrant is bound to a named SAS revision
- **scope:** §14, §64.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** every committed `WAR.json` carries `format_basis.sas_revision`
  and `sas_digest`; the `composition_revision_digest` of every Warrant is
  unchanged by this; `war status` shows the revision on the Release axis.

## Gate Adequacy

Required at `controlled`.

**Adversarial question:** can the SAS be changed without anyone deciding to
change it? The attacks: a byte edited in place; a row silently deleted from
§106; a row renumbered; a revision accepted by the agent that proposed it;
an accepted record edited by hand to a different digest.

**Executed attacks:** four plants in `conformance/plant.sh` and six unit tests
in `openwarrant-core::sas`:

- one byte edited in §106's heading under an unchanged record →
  `sas.digest-drift`, naming both digests
- a candidate document with `WAR-SAS-RQ-042` deleted → `war sas diff` refuses
  as `sas.diff.removed`, naming the id
- a candidate with `WAR-SAS-RQ-999` appended → reported as `sas.diff.added`,
  not refused
- an acceptance response naming `claude` → refused; the record stays
  `proposed`
- unit: a second acceptance of an accepted revision is `Immutable`; an agent
  acceptance is refused; an architecture-changing acceptance without an
  `adr_ref` is refused; a removed id fails `check_stability`; a bad digest
  and an accepted-without-acceptance record fail `validate`

The renumbering attack is the removal attack: a renumbered row is one id
removed and one added, and the removal is what is refused.

One consequence found by executing rather than by reading: pinning the SAS
into the format basis moved every contract digest, and four exact parent
citations had to be re-cited (recorded in the basis atom). The relations
check caught it before the gate did.

- **outcome:** counterexample_found, gate_added

## Residual Risk

- The record is a TOML file in the repository, editable by anyone with a
  commit. Immutability is structural in the type and checked on load; it is
  not cryptographic. §101.1's controlled Knowledge Fabric document is where
  that changes, and it is not built here.
- A retitle is treated as architecture-changing (needs an ADR) but not as a
  removal (not refused). A retitle that inverts a requirement's meaning
  passes the stability check and is caught only by whoever reads the ADR.
- Proposing `0.1.0-draft.1` moves every workspace basis digest. Any
  out-of-tree consumer that cached one is wrong from that commit on, and
  this repository cannot know who that is.
