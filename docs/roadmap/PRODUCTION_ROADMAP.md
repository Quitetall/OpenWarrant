# OpenWarrant Production Roadmap

The SAS says *what* the system becomes. This says *when and in what order* (§6.3),
and what each release gate means.

Governing specification: `docs/sas/WAR_Software_Architecture_Specification.md`
v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.

## Alpha reached — and what that does and does not mean

**All 40 alpha Warrants are resolved as of 2026-08-20.** Measured, not asserted:

```
cargo xtask gate    PASS, 7/7 steps
conformance/plant.sh    36 planted violations, 36 rejected by their intended control
war check --generated   212 pass · 2 warn · 0 unknown · 0 error
cargo test --workspace  479 tests
```

Alpha means **every SAS capability exists and OpenWarrant implements its side of
every protocol.** It does not mean the system has been used.

Read the four honest limits below before treating this as a finished product:

1. **Nothing has run against a real neighbour.** Katana, BLUT, Knowledge Fabric
   and Liminal have no checkout on this host. The seams are typed, tested, and
   unexercised. A protocol proven only against its own tests is not proven.
2. **The execution plane is not wired into the binary at all.** This is
   stronger than "has no plants", and it was measured on 2026-08-20 rather than
   assumed: of twenty alpha types sampled — `Admissibility`, `Independence`,
   `EvidenceItem`, `Observation`, `Judgment`, `GateReceipt`, `KatanaReceipt`,
   `ResolutionChecks`, `PreflightReceipt`, `ClaimGraph`, `ActionEnvelope`,
   `PortableExport` and the rest — **twenty are unreachable from `war` and the
   compiler.** §40.7's six prohibited substitutions are enforced by a function
   nothing calls.

   The alpha claim stands as worded: the capability exists, typed and tested.
   But a reader could reasonably infer that `war check` enforces these rules on
   a corpus, and it does not. Wiring them into the check path, and putting a
   plant on each, is OW-WAR-0046's first deliverable.
3. **Two adequacy reviews report no executed attacks, and say so.** OW-WAR-0008
   and OW-WAR-0017 are the two `war check` still warns about. That is the check
   working: the state model and the epistemic classes have nothing to attack
   until real authorizations and judgments exist.
4. **This repository authored and verified itself.** Every Warrant here has
   `independence: none` in substance — one actor, every role. §27.4 is explicit
   that role separation by one person is not organizational independence, and
   `Independence::none` satisfies nothing, including `basic`.

Beta is where 1 and 2 close. Release is where the hardening Warrants are written.

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
| 0 — Telemetry shim | alpha (vocabulary) / beta (measured) | **resolved** 2026-08-20 (OW-WAR-0039); exit open, see OW-WAR-0041 |
| 1 — File-native WAR compiler | alpha | **resolved** (partial — see below) |
| — milestones and stages (OW-WAR-0007) | alpha | **resolved** 2026-08-19 |
| — state model (OW-WAR-0008) | alpha | **resolved** 2026-08-19 |
| — contract revisions (OW-WAR-0009) | alpha | **resolved** 2026-08-19 |
| — acceptance obligations (OW-WAR-0016) | alpha | **resolved** 2026-08-19 |
| — epistemic classes (OW-WAR-0017) | alpha | **resolved** 2026-08-19 |
| — contract-adequacy review (OW-WAR-0018) | alpha | **resolved** 2026-08-19 |
| — gate registry, qualification, binding (OW-WAR-0019) | alpha | **resolved** 2026-08-19 |
| — gate run semantics + invalidation (OW-WAR-0020) | alpha | **resolved** 2026-08-19 |
| — verifier independence (OW-WAR-0021) | alpha | **resolved** 2026-08-20 |
| — resolution, dispute, annulment (OW-WAR-0022) | alpha | **resolved** 2026-08-20 |
| — dispatch compilation + actor projection (OW-WAR-0023) | alpha | **resolved** 2026-08-20 |
| — stage submission + attempt semantics (OW-WAR-0024) | alpha | **resolved** 2026-08-20 |
| — blockers, deviations, decisions, gaps (OW-WAR-0025) | alpha | **resolved** 2026-08-20 |
| — Katana runtime seam + receipts (OW-WAR-0026) | alpha | **resolved** 2026-08-20 |
| — BLUT adapter: lowering + lineage receipt (OW-WAR-0027) | alpha | **resolved** 2026-08-20 |
| — KF typed actions + controlled envelope (OW-WAR-0028) | alpha | **resolved** 2026-08-20 |
| — Liminal adapter + measured parity (OW-WAR-0040) | alpha | **resolved** 2026-08-20 |
| — portable preservation + round trip (OW-WAR-0030) | alpha | **resolved** 2026-08-20 |
| — local draft journal (OW-WAR-0031) | alpha | **resolved** 2026-08-20 |
| — ADR federation: relations, supersession, currency (OW-WAR-0006) | alpha | **resolved** 2026-08-20 |
| — KF registration + global identity allocation (OW-WAR-0029) | alpha | **resolved** 2026-08-20 |
| — schema pack + protocol versioning (OW-WAR-0032) | alpha | **resolved** 2026-08-20 |
| — the remaining read projections (OW-WAR-0033) | alpha | **resolved** 2026-08-20 |
| — agent protocol + Draft Proposal validation (OW-WAR-0034) | alpha | **resolved** 2026-08-20 |
| — `war plan` and the interview loop (OW-WAR-0035) | alpha | **resolved** 2026-08-20 |
| — normative-decision detection (OW-WAR-0036) | alpha | **resolved** 2026-08-20 |
| — `war diff`: semantic difference (OW-WAR-0037) | alpha | **resolved** 2026-08-20 |
| — existing ADR importer (OW-WAR-0038) | alpha | **resolved** 2026-08-20 |
| — telemetry + untracked-work detection (OW-WAR-0039) | alpha | **resolved** 2026-08-20 |
| — autonomy envelope + amendment records (OW-WAR-0010) | alpha | **resolved** 2026-08-20 |
| — prerequisites and Preflight (OW-WAR-0011) | alpha | **resolved** 2026-08-20 |
| — context model, manifest, trust classes (OW-WAR-0012) | alpha | **resolved** 2026-08-20 |
| — SAS + Roadmap traceability (OW-WAR-0013) | alpha | **resolved** 2026-08-20 |
| — rationale model, assumptions, unknowns (OW-WAR-0014) | alpha | **resolved** 2026-08-20 |
| — deliverables, artifacts, provenance (OW-WAR-0015) | alpha | **resolved** 2026-08-20 |
| 2 — Agent planner | alpha (protocol) / beta (live) | **resolved** 2026-08-20 (OW-WAR-0034, 0035); exit open, see OW-WAR-0042 |
| 3 — ADR federation | alpha | partial |
| 4 — Knowledge Fabric registration | alpha (protocol) / beta (live) | not started |
| 5 — Dispatch and Katana execution | alpha (protocol) / beta (live) | not started |
| 6 — Gate Registry and assurance case | alpha | **resolved** 2026-08-20 (OW-WAR-0019, 0020); exit open, see OW-WAR-0046 |
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
| ~~OW-WAR-0006~~ | ~~Complete ADR federation: relations, supersession, currency~~ **RESOLVED** | §19.4, §21 | RQ-024, RQ-025 |
| ~~OW-WAR-0007~~ | ~~Milestones and stages: parse, validate, named typed ports~~ **RESOLVED** | §23 | RQ-040, RQ-041 |
| ~~OW-WAR-0008~~ | ~~The state model: phase, condition, outcome, currency, standing~~ **RESOLVED** | §24 | RQ-032 |
| ~~OW-WAR-0009~~ | ~~Contract revisions and immutability~~ **RESOLVED** | §28, §29 | RQ-030, RQ-031, RQ-033, RQ-034 |
| ~~OW-WAR-0010~~ | ~~Autonomy envelope and amendment records~~ **RESOLVED** | §30, §31 | — |
| ~~OW-WAR-0011~~ | ~~Prerequisites and Preflight~~ **RESOLVED** | §32 | RQ-035 |
| ~~OW-WAR-0012~~ | ~~Context model, context manifest, trust classes~~ **RESOLVED** | §33 | — |
| ~~OW-WAR-0013~~ | ~~SAS and Roadmap traceability validation~~ **RESOLVED** | §34 | RQ-022 |
| ~~OW-WAR-0014~~ | ~~Rationale model, assumptions, and unknowns~~ **RESOLVED** | §35, §36 | — |
| ~~OW-WAR-0015~~ | ~~Deliverables, artifacts, and provenance~~ **RESOLVED** | §37 | — |

### Assurance — the reason the system exists

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| ~~OW-WAR-0016~~ | ~~Acceptance obligations and bounded claims~~ **RESOLVED** | §38 | RQ-050, RQ-051 |
| ~~OW-WAR-0017~~ | ~~Epistemic classes: evidence, observation, inference, judgment, resolution~~ **RESOLVED** | §40, §41, §42 | RQ-052 |
| ~~OW-WAR-0018~~ | ~~Contract-adequacy review, structurally checked~~ **RESOLVED** | §39 | RQ-055 |
| ~~OW-WAR-0019~~ | ~~Gate Registry: definitions, qualification, bindings~~ **RESOLVED** | §43 | RQ-056 |
| ~~OW-WAR-0020~~ | ~~Gate Run semantics, askability, and invalidation~~ **RESOLVED** | §44, §45 | RQ-054, RQ-057 |
| ~~OW-WAR-0021~~ | ~~Verifier independence~~ **RESOLVED** | §46 | RQ-053 |
| ~~OW-WAR-0022~~ | ~~Resolution, dispute, and annulment~~ **RESOLVED** | §56, §57 | RQ-058, RQ-059 |

### Execution

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| ~~OW-WAR-0023~~ | ~~Stage Dispatch compilation and actor-specific projection~~ **RESOLVED** | §47 | RQ-042, RQ-043 |
| ~~OW-WAR-0024~~ | ~~Stage Submission and attempt semantics~~ **RESOLVED** | §51, §52 | RQ-045 |
| ~~OW-WAR-0025~~ | ~~Blockers, deviations, decision proposals, discovered gaps~~ **RESOLVED** | §53 | — |
| ~~OW-WAR-0026~~ | ~~Katana runtime seam, capabilities, and receipts~~ **RESOLVED** | §48 | RQ-044, RQ-062 |
| ~~OW-WAR-0027~~ | ~~BLUT adapter: PlanSpec lowering and lineage receipt~~ **RESOLVED** | §49 | RQ-063 |

### Federation and preservation

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| ~~OW-WAR-0028~~ | ~~Knowledge Fabric typed actions and controlled-action envelope~~ **RESOLVED** | §67 | RQ-076 |
| ~~OW-WAR-0029~~ | ~~KF registration, global identity allocation, federation~~ **RESOLVED** | §12, §83 | RQ-003, RQ-004, RQ-005 |
| ~~OW-WAR-0030~~ | ~~Portable preservation: one-file export and round trip~~ **RESOLVED** | §68 | RQ-082, RQ-083, RQ-084 |
| ~~OW-WAR-0031~~ | ~~Local draft journal~~ **RESOLVED** | §66 | — |
| ~~OW-WAR-0032~~ | ~~Schema pack generation and protocol versioning~~ **RESOLVED** | §64, §69, §83.4 | — |
| ~~OW-WAR-0033~~ | ~~The remaining read projections~~ **RESOLVED** | §17.5 | — |

### Planning and migration

| WAR | Title | SAS | Requirements |
|---|---|---|---|
| ~~OW-WAR-0034~~ | ~~Agent protocol and Draft Proposal validation~~ **RESOLVED** | §74, §75 | RQ-072 |
| ~~OW-WAR-0035~~ | ~~`war plan` and the interview loop~~ **RESOLVED** | §71.3, §71.4 | RQ-071 |
| ~~OW-WAR-0036~~ | ~~Normative-decision detection and proposed-ADR generation~~ **RESOLVED** | §19.2, §74.7 | RQ-020, RQ-073 |
| ~~OW-WAR-0037~~ | ~~`war diff`: semantic difference between revisions~~ **RESOLVED** | §71.10 | — |
| ~~OW-WAR-0038~~ | ~~Existing ADR importer, preserving unknown classes~~ **RESOLVED** | §96, §97 | — |
| ~~OW-WAR-0039~~ | ~~Telemetry, unit economics, and untracked-work detection~~ **RESOLVED** | §94, §95 | — |
| ~~OW-WAR-0040~~ | ~~Liminal adapter and measured parity harness~~ **RESOLVED** | §82 | RQ-061 |

**Alpha = OW-WAR-0006 through OW-WAR-0040 resolved, plus the three carried-forward
Phase 1 milestones.** 35 Warrants.

## Beta Warrants

Nine, authored 2026-08-20. The spine is the SAS's own §98 phase **Exit**
criteria: alpha built every phase's Deliver list, and the Exits are behavioural
claims that require a real run. None has been discharged.

| WAR | Title | SAS | Exit discharged |
|---|---|---|---|
| OW-WAR-0041 | Real telemetry distributions, with a baseline | §94, §95, §100 | Phase 0 |
| OW-WAR-0042 | A vague request becomes a reviewable draft | §74, §75, §91.8 | Phase 2 |
| OW-WAR-0043 | Migrate the LamQuant ADR corpus, fabricating nothing | §96, §97, §91.5 | Phase 3 |
| OW-WAR-0044 | KF is institutional authority, Git stays Source Holder | §67, §83, §91.13 | Phase 4 |
| OW-WAR-0045 | A stateless actor executes one Dispatch | §47, §48, §51, §91.7, §91.9 | Phase 5 |
| OW-WAR-0046 | A delivery closes only through bounded proof | §43, §44, §56, §91.10–§91.12 | Phase 6 |
| OW-WAR-0047 | Compatible WARs execute without duplicating BLUT | §49, §91.7 | Phase 7 |
| OW-WAR-0048 | Measured adapter parity and the two-host canonical run | §82, §91.1 | Phase 8 |
| OW-WAR-0049 | Close the alpha residue and one false claim | §91.2, §28, §31 | — |

Every one of §91's 95 conformance tests now has an owning Warrant. §91 runs
§91.1 through **§91.13**; subsections §91.7–§91.13 (tests 43–95) had zero
citations anywhere when beta was authored.

**Sequencing.** OW-WAR-0046 is numbered sixth and should run first: while the
validators are unreachable from the binary, every other beta obligation is
unverifiable and a plant against an unreachable rule cannot fail. OW-WAR-0041 is
the only time-critical one — §100 says "measurably reduces", and a baseline not
taken at the alpha commit cannot be taken later.

**What is actionable now.** BLUT (`training/engine`) and Knowledge Fabric are
both checked out on the development host, so OW-WAR-0043, 0044 and 0047 need no
new infrastructure. Katana and Liminal are repositories under the same owner but
are not cloned; OW-WAR-0045 and OW-WAR-0048 carry §36.3 blocking unknowns naming
exactly what is missing, so they block readiness rather than passing quietly.

**One thing going public already fixed.** §91.1 test 1 ("two hosts →
byte-identical canonical IR") was reported as blocked on hardware. It is not:
`release.yml` already carries `x86_64-unknown-linux-gnu` and
`aarch64-apple-darwin`, and public repositories get free minutes. Different OS,
different architecture, zero cost. OW-WAR-0048 owns it.

## Release Warrants

- Phase 9: signatures, audit checkpoints, controlled evidence custody, physical
  test profile, independent human workflow, invalidation propagation, regulatory
  mapping (§98 Phase 9).
- Security boundary hardening (§55, §87), performance (§88), observability (§89),
  transactional behaviour (§86).
- Phase 10: contractor Work Order profile, gated on the legal, finance and QMS
  decisions §98 names — not an engineering decision.
- The AGPL-3.0 → Apache-2.0 relicense and public launch (`RELICENSING.md`).


### Adequacy reviews — the state as of 2026-08-20

This section said "12 of 14 controlled Warrants record no §39.2 outcome" from
2026-08-19 until this correction. That was true when written and false by the
time alpha closed, which is the failure mode the whole document is about. The
numbers below are what `war check` reports today; if they disagree with the tool,
the tool is right.

- **8 of 22** `controlled` Warrants lack a §39.2 outcome — the eight beta
  Warrants authored 2026-08-20, whose reviews have not happened yet.
- **10 of 22** have executed no §39.3 attacks: the eight beta Warrants, plus
  OW-WAR-0008 (state model) and
  OW-WAR-0017 (epistemic classes). Both say so in their reviews and explain why —
  there is nothing to attack until real authorizations and judgments exist. These
  are the only two warnings `war check` emits.

Missing attacks stay a WARN rather than an ERROR: §39.3 says attacks SHOULD be
run "where economical", and a SHOULD promoted to a hard failure is the first rule
anyone disables.

The rule was written to catch this repository first, and it did — twice.
OW-WAR-0023 had passed RQ-055 for two weeks with a section that never asked
§39.1's question, because the old check was a substring search for the word
"adequacy". Then the replacement's own absence-phrase allowlist was wrong three
times, hiding warnings for the Warrants being most honest, until attacks were
counted structurally instead.

### What "resolved" means for a module the corpus cannot yet exercise

Several alpha Warrants deliver machinery that `war check` does not yet consult,
because the record it would govern does not exist in this repository. Independence
declarations, authorization records, resolutions, disputes and monitors all arrive
as DATA with the local journal (OW-WAR-0031) and Knowledge Fabric federation
(OW-WAR-0028).

Those Warrants are resolved in the alpha sense — the capability exists, is typed,
and is unit-tested against the SAS's own vocabulary — and are NOT resolved in the
sense of being exercised on real records. The plant battery is the honest measure
here: a rule with no plant is a rule nothing has tried to break. Current plants
cover the record-level rules (§16, §19, §23, §28, §38, §39, §43, §44); the
execution-plane rules are unit-tested only.

Beta is where these meet real data. Until then, no gate count should be read as
evidence that resolution semantics have been exercised end to end.

## How this document stays true

It is a source atom, not a projection: it is hand-maintained. The generated
Warrant Overview (`docs/warrants/generated/WARRANT_OVERVIEW.md`) is compiled from
the Warrants themselves and is the authority on what exists and what state it is
in. Where the two disagree, the Overview is right and this file is stale.
