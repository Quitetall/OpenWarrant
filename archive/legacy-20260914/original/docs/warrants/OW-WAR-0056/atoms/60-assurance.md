---
schema: oh.war/atom/v1
warrant_uuid: 01a06025-b26f-7950-be1c-c830b2a215af
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — every §47.1 field is present and required ones are enforced
- **scope:** §47.1's schema, all twenty-three top-level fields.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a unit test enumerates §47.1's field names against the
  serialized dispatch; `validate` refuses an empty `workspace_basis_ref`,
  `context_manifest_ref`, `submission_schema_ref` and a wrong `api_version`.

### OBL-002 — a dispatch is byte-deterministic for fixed ids
- **scope:** §47.2 "produce deterministic canonical bytes".
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two compilations of one stage with the same `dispatch_id` and
  `attempt_id` produce identical canonical bytes and identical
  `dispatch_digest`; changing the context manifest changes both.

### OBL-003 — a required normative source cannot be omitted
- **scope:** §47.2, §33.6.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a context manifest that omits a required atom makes
  `compile_dispatch` refuse, naming the atom; a unit test and a plant.

### OBL-004 — `war dispatch` emits a packet a receipt can bind to
- **scope:** §47.1, §48.4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war dispatch OW-WAR-0047 STAGE-002` — the one corpus stage with an
  `executor_ref` — prints canonical JSON
  whose `dispatch_digest` is what `KatanaReceipt::validate` compares against.

### OBL-005 — a repair without prior failure evidence is refused
- **scope:** §52.3, §47.2 "include prior failure evidence for repair".
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war dispatch … --attempt-kind repair` with no evidence is
  refused as a repair, not degraded to an initial attempt; a plant.

## Gate Adequacy

Required at `controlled`.

**Adversarial question:** can a dispatch be produced that a stateless actor
would execute against the wrong contract? The attacks: a stage id that names
no stage; a required atom silently dropped from context; a repair that cannot
see what failed; a dispatch whose digest was computed over itself; two
compilations of one stage that differ.

**Executed attacks:** four plants in `conformance/plant.sh` and four unit tests
in `openwarrant-compiler::dispatch`, each rejected by its intended control:

- `war dispatch OW-WAR-0047 STAGE-099` → "no stage", naming the three that exist
- `war dispatch OW-WAR-0021 STAGE-003` → "declares no executor_ref", refused
  rather than dispatched under the WAR id
- `war dispatch OW-WAR-0047 STAGE-002 --attempt-kind repair` with no evidence →
  refused as a repair (§52.3), not degraded to an initial attempt
- positive: `war dispatch OW-WAR-0047 STAGE-002` emits one line of canonical
  JSON carrying `oh.war/stage-dispatch/v1` and a `dispatch_digest`, with the
  report on stderr so stdout is the packet and nothing else
- unit: two compilations with fixed ids are byte-identical; a changed context
  manifest changes both digests
- unit: the `dispatch_digest` recomputes from the packet with the field blank
- unit: a required atom absent from the manifest and not recorded as omitted is
  refused by name; recorded as omitted, refused by `validate` (§47.2)
- unit: an unbound stage and an evidence-free repair are refused

The fixed-point attack — a digest computed over a packet already containing
it — is held by construction: the digest is computed with the field empty
and written in after, and the recompute test would fail otherwise.

One counterexample was found in the test battery, not the compiler: the first
fixture wrote ports as nested mappings, which the restricted atom reader
(OW-ADR-0003) refuses. Ports are `"name:type"` scalars, as the corpus writes
them.

- **outcome:** counterexample_found, gate_added

## Residual Risk

- Context is selected from the Warrant's atoms only. A stage that needs a
  source outside the Warrant — a gate fixture, an external spec — gets a
  dispatch that is complete by §33.6 and incomplete in fact. The compiler
  cannot know what it was not told about.
- The capability policy is a placeholder reference. A runtime that enforces
  capabilities against it will find nothing to enforce, which is honest and
  is not authorization.
- No actor has executed a dispatch. §47.3's promise — different projections,
  one contract — is held by `same_normative_contract_as` and by no observed
  actor.
