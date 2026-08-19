# OpenWarrant Production Roadmap

The SAS says *what* the system becomes. This says *when and in what order* (§6.3),
and what each release gate means.

Governing specification: `docs/sas/WAR_Software_Architecture_Specification.md`
v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.

## Release gates

| Gate | Means | Complete when |
|---|---|---|
| **Alpha** | Feature complete. Every SAS capability exists and OpenWarrant implements its side of every protocol. | All alpha Warrants resolved |
| **Beta** | Tested. Integration against the live Katana, BLUT, Knowledge Fabric and Liminal systems; conformance across two real hosts. | All beta Warrants resolved |
| **Release** | Hardened. High-assurance controls, security boundary, performance, observability, and the contractor profile. | All release Warrants resolved |

**Alpha explicitly does NOT include** hardening, performance work, the security
boundary beyond what correctness requires, signature infrastructure, or the
contractor Work Order profile. Those are named below and deferred on purpose so
that "feature complete" is a claim about capability, not about production
fitness. A capability that exists but has not been hardened is alpha; calling it
release would be the kind of overstatement this system exists to prevent.

**The adapters are alpha on our side only.** §11.2 makes OpenWarrant "the
protocol and compiler surface" — so compiling a Dispatch, lowering a stage graph,
and defining a receipt schema are alpha. Executing against a live kernel is beta,
because Katana and Liminal have no checkout on the development host and a
protocol proven only against a mock is not proven.

## Phase mapping (SAS §98)

| Phase | Gate | Status |
|---|---|---|
| 0 — Telemetry shim | alpha | not started |
| 1 — File-native WAR compiler | alpha | **resolved** (partial — see below) |
| — milestones and stages (OW-WAR-0007) | alpha | **resolved** 2026-08-19 |
| — state model (OW-WAR-0008) | alpha | **resolved** 2026-08-19 |
| — contract revisions (OW-WAR-0009) | alpha | **resolved** 2026-08-19 |
| — acceptance obligations (OW-WAR-0016) | alpha | **resolved** 2026-08-19 |
| — epistemic classes (OW-WAR-0017) | alpha | **resolved** 2026-08-19 |
| — contract-adequacy review (OW-WAR-0018) | alpha | **resolved** 2026-08-19 |
| — gate registry, qualification, binding (OW-WAR-0019) | alpha | **resolved** 2026-08-19 |
| — gate run semantics + invalidation (OW-WAR-0020) | alpha | **resolved** 2026-08-19 |
| 2 — Agent planner | alpha | not started |
| 3 — ADR federation | alpha | partial |
| 4 — Knowledge Fabric registration | alpha (protocol) / beta (live) | not started |
| 5 — Dispatch and Katana execution | alpha (protocol) / beta (live) | not started |
| 6 — Gate Registry and assurance case | alpha | not started |
| 7 — BLUT adapter | alpha (protocol) / beta (live) | not started |
| 8 — Liminal production compiler | alpha (protocol) / beta (live) | not started |
| 9 — High-assurance controls | release | not started |
| 10 — Contractor Work Order profile | release | not started, and gated on legal/finance/QMS decisions the SAS names |

### Phase 1 is resolved but not whole

Recorded here rather than left implied. Three milestones in the Phase 1 Warrants
are **not met**, and they are carried forward as scope of the alpha Warrants
below rather than quietly dropped:

- ~~**OW-WAR-0002 M2** — §91.2 test 12 (composition cycle) has unit tests but no
  plant against the shipped binary.~~ **CLOSED 2026-08-19** by the
  `composition cycle (self-parent)` plant.
- **OW-WAR-0005 M2** — §91.2 test 10 (a generated atom cannot be edited through
  an authored-source command) is not implemented.
- **OW-WAR-0005 M4** — bootstrap closure was declared, then the very next unit of
  work (the ADR Overview, commit `3678455`) shipped with no Warrant. That is
  untracked work under §95, committed against this repository's own obligation.

## Alpha Warrants

Every SAS requirement in §106 that is not already met maps to exactly one Warrant
here. `roadmap://` refs in manifests resolve to the identifiers in this table.

### Core protocol — the semantics everything else rests on

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| OW-WAR-0006 | Complete ADR federation: relations, supersession, currency | §19.4, §21 | RQ-024, RQ-025 |
| ~~OW-WAR-0007~~ | ~~Milestones and stages: parse, validate, named typed ports~~ **RESOLVED** | §23 | RQ-040, RQ-041 |
| ~~OW-WAR-0008~~ | ~~The state model: phase, condition, outcome, currency, standing~~ **RESOLVED** | §24 | RQ-032 |
| ~~OW-WAR-0009~~ | ~~Contract revisions and immutability~~ **RESOLVED** | §28, §29 | RQ-030, RQ-031, RQ-033, RQ-034 |
| OW-WAR-0010 | Autonomy envelope and amendment records | §30, §31 | — |
| OW-WAR-0011 | Prerequisites and Preflight | §32 | RQ-035 |
| OW-WAR-0012 | Context model, context manifest, trust classes | §33 | — |
| OW-WAR-0013 | SAS and Roadmap traceability validation | §34 | RQ-022 |
| OW-WAR-0014 | Rationale model, assumptions, and unknowns | §35, §36 | — |
| OW-WAR-0015 | Deliverables, artifacts, and provenance | §37 | — |

### Assurance — the reason the system exists

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| ~~OW-WAR-0016~~ | ~~Acceptance obligations and bounded claims~~ **RESOLVED** | §38 | RQ-050, RQ-051 |
| ~~OW-WAR-0017~~ | ~~Epistemic classes: evidence, observation, inference, judgment, resolution~~ **RESOLVED** | §40, §41, §42 | RQ-052 |
| ~~OW-WAR-0018~~ | ~~Contract-adequacy review, structurally checked~~ **RESOLVED** | §39 | RQ-055 |
| ~~OW-WAR-0019~~ | ~~Gate Registry: definitions, qualification, bindings~~ **RESOLVED** | §43 | RQ-056 |
| ~~OW-WAR-0020~~ | ~~Gate Run semantics, askability, and invalidation~~ **RESOLVED** | §44, §45 | RQ-054, RQ-057 |
| OW-WAR-0021 | Verifier independence | §46 | RQ-053 |
| OW-WAR-0022 | Resolution, dispute, and annulment | §56, §57 | RQ-058, RQ-059 |

### Execution

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| OW-WAR-0023 | Stage Dispatch compilation and actor-specific projection | §47 | RQ-042, RQ-043 |
| OW-WAR-0024 | Stage Submission and attempt semantics | §51, §52 | RQ-045 |
| OW-WAR-0025 | Blockers, deviations, decision proposals, discovered gaps | §53 | — |
| OW-WAR-0026 | Katana runtime seam, capabilities, and receipts | §48 | RQ-044, RQ-062 |
| OW-WAR-0027 | BLUT adapter: PlanSpec lowering and lineage receipt | §49 | RQ-063 |

### Federation and preservation

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| OW-WAR-0028 | Knowledge Fabric typed actions and controlled-action envelope | §67 | RQ-076 |
| OW-WAR-0029 | KF registration, global identity allocation, federation | §12, §83 | RQ-003, RQ-004, RQ-005 |
| OW-WAR-0030 | Portable preservation: one-file export and round trip | §68 | RQ-082, RQ-083, RQ-084 |
| OW-WAR-0031 | Local draft journal | §66 | — |
| OW-WAR-0032 | Schema pack generation and protocol versioning | §64, §69, §83.4 | — |
| OW-WAR-0033 | The remaining read projections | §17.5 | — |

### Planning and migration

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| OW-WAR-0034 | Agent protocol and Draft Proposal validation | §74, §75 | RQ-072 |
| OW-WAR-0035 | `war plan` and the interview loop | §71.3, §71.4 | RQ-071 |
| OW-WAR-0036 | Normative-decision detection and proposed-ADR generation | §19.2, §74.7 | RQ-020, RQ-073 |
| OW-WAR-0037 | `war diff`: semantic difference between revisions | §71.10 | — |
| OW-WAR-0038 | Existing ADR importer, preserving unknown classes | §96, §97 | — |
| OW-WAR-0039 | Telemetry, unit economics, and untracked-work detection | §94, §95 | — |
| OW-WAR-0040 | Liminal adapter and measured parity harness | §82 | RQ-061 |

**Alpha = OW-WAR-0006 through OW-WAR-0040 resolved, plus the three carried-forward
Phase 1 milestones.** 35 Warrants.

## Beta Warrants

Written when alpha closes, not before. Scope, so it is not rediscovered:

- Live integration against Katana, BLUT, Knowledge Fabric, and Liminal.
- §91.1 test 1 on two genuinely different hosts, not two runs on one.
- The full upstream `es6-numbers` corpus rather than the pinned boundary cases.
- Conformance across the whole §91 suite — 95 tests.
- Migration of the LamQuant ADR corpus, executed and verified.

## Release Warrants

- Phase 9: signatures, audit checkpoints, controlled evidence custody, physical
  test profile, independent human workflow, invalidation propagation, regulatory
  mapping (§98 Phase 9).
- Security boundary hardening (§55, §87), performance (§88), observability (§89),
  transactional behaviour (§86).
- Phase 10: contractor Work Order profile, gated on the legal, finance and QMS
  decisions §98 names — not an engineering decision.
- The AGPL-3.0 → Apache-2.0 relicense and public launch (`RELICENSING.md`).


### Known gap, recorded rather than closed (OW-WAR-0018)

`war check` now reports, on this repository's own corpus:

- **12 of 14** `controlled` Warrants carry an adequacy review that records no
  §39.2 outcome and has executed no §39.3 attacks. Both are WARN, not ERROR:
  §39.3 says attacks SHOULD be run "where economical", and a SHOULD promoted to
  a hard failure is the first rule anyone disables.
- These 12 are Warrants whose work has not been executed yet. There is nothing
  to attack until there is an implementation, so the honest record is a review
  that asked its question and stopped. They close as their Warrants resolve.
- The rule was written to catch this repository first. It did: OW-WAR-0023 had
  passed RQ-055 for two weeks with a section that never asked §39.1's question,
  because the old check was a substring search for the word "adequacy". That
  substring search is deleted, and a repository-wide grep for its call site
  returns nothing.

## How this document stays true

It is a source atom, not a projection: it is hand-maintained. The generated
Warrant Overview (`docs/warrants/generated/WARRANT_OVERVIEW.md`) is compiled from
the Warrants themselves and is the authority on what exists and what state it is
in. Where the two disagree, the Overview is right and this file is stale.
