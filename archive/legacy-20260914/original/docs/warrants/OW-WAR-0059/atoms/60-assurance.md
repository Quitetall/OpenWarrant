---
schema: oh.war/atom/v1
warrant_uuid: 01a060c6-64a5-71e1-b994-133d3c1e19d2
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an edited receipt is not a receipt
- **scope:** §44.6, §51.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a unit test flips a sealed receipt's verdict and
  `admissibility` refuses it as not recomputing; a plant does the same to a
  committed receipt and `war check` refuses it as `evidence.receipt-invalid`.

### OBL-002 — a receipt for an earlier contract is not evidence about this one
- **scope:** §56.1 requirement 1 and 5 together.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a unit test binds a receipt to another digest and
  `admissible_runs` returns nothing; a plant edits an atom of a Warrant with
  recorded evidence and `war resolve --dry-run` reports requirement 5 unmet
  again, while `war check` reports `evidence.stale-binding`.

### OBL-003 — requirement 5 answers from tracked inputs only
- **scope:** §59.2; the reproducibility rule OW-WAR-0055 established.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the projection's source has no reader of the receipts path;
  `war check --generated` passes in CI on the committed projection with
  three Warrants reading requirement 5 met; the caveat names `gate-runs/`.

### OBL-004 — an agent cannot resolve
- **scope:** §27.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant ingests a resolution response naming `claude` for a
  Warrant with all thirteen met and is refused as `resolution.agent`; no
  `resolution.toml` is written.

### OBL-005 — `satisfied` cannot be signed over unestablished obligations
- **scope:** §38.6, §56.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant ingests `common_outcome = "satisfied"` for a Warrant
  whose obligations are not established and is refused as
  `resolution.outcome-unsupported`, naming the obligations.

### OBL-006 — a resolution of a moved contract is stale, not silently current
- **scope:** §45, §56.1 requirement 1.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant writes a well-formed `resolution.toml` binding a
  different digest and `war check` refuses it as `resolution.stale`; the
  projection reports the record with `binds_current_contract = false` and
  derives no rung from it.

### OBL-007 — two Warrants carry admissible evidence
- **scope:** §44.6.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check` reports `evidence.admissible` for OW-WAR-0010
  and OW-WAR-0020; `war resolve --dry-run` reports all thirteen met for each;
  `war resolve <alias>` emits a request with `requirements_met = true` and
  `satisfied` among the permitted outcomes.

## Gate Adequacy

Required at `controlled`.

**Adversarial question:** can requirement 5 or a resolution be satisfied by
something other than a gate that ran against this contract and a human who
signed for it? The attacks: a receipt edited to `pass`; a run file edited to
`pass` beside an honest receipt; a receipt from before the contract moved; a
resolution signed by the agent; `satisfied` signed over unestablished
obligations; a resolution left in place after the contract moved; a second
resolution ingested over the first.

**Executed attacks:** seven unit tests in `openwarrant-cli::evidence` and
six plants in `conformance/plant.sh`:

- a committed receipt's verdict flipped to `pass` → `evidence.receipt-invalid`
  ("does not recompute")
- a run file's verdict flipped to `pass` beside a receipt saying `fail` →
  `evidence.receipt-invalid` ("says verdict")
- an atom edited after evidence was recorded → `evidence.stale-binding`, and
  `war resolve --dry-run` unmet on requirement 5
- a resolution response naming `claude` → `resolution.agent`, nothing written
- `satisfied` for a Warrant with an unestablished obligation →
  `resolution.outcome-unsupported`, naming it
- a `resolution.toml` binding another digest → `resolution.stale`
- unit: sealed-and-bound admissible; edited does not reseal; other contract
  is a record not evidence; no receipt; run and receipt disagree; failing
  run however well sealed; no contract to bind to

Two consequences found by executing rather than by reading: minting into
the tracked directory required the runner to take an output directory — a
copy step, the obvious design, produces a receipt whose stream refs point at
the gitignored path or, if rewritten, no longer reseals. And a receipt for
one Warrant drifts the projection the next Warrant's gate run checks, so
`war compile` has to run between recordings (recorded in the basis).

- **outcome:** counterexample_found, gate_added

## Residual Risk

- The receipt seals its own fields; it does not seal the stdout and stderr
  files it references. Those are committed beside it and diffable, but an
  edited stdout with an untouched receipt is caught by a reviewer, not a
  check. A `stdout_digest` field is a §44.6 amendment, not this Warrant.
- A recorded run is bound to the contract, so an edit to a non-contract atom
  keeps it admissible. If the Basis rule (§14) is later read to require
  workspace-basis binding, every recorded run is stale at once.
- Two resolution requests are emitted and none is signed here. If the
  owner signs them with wording the agent drafted, the record says so; the
  decision is still theirs, and the projection cannot tell the difference.
