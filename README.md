# OpenWarrant

**WAR — Work Authorization Record.** The human noun is a **Warrant**; the CLI is
`war`.

A Warrant is one semantic work object compiled from ordered source atoms into
role-specific projections. It carries the authority for a bounded piece of work,
the basis it was authorized on, what was actually attempted, what independent
methods observed, and what the organization concluded — as one auditable record
rather than a trail of documents that disagree.

> A WAR does not claim that work is correct because an agent stopped working or a
> command exited zero.

## Status

**Alpha complete. Beta open.**

Alpha means every SAS capability exists and OpenWarrant implements its side of
every protocol — 49 Warrants, 40 of them resolved. It does **not** mean the
system has been used, and the distinction is the whole point of the next
paragraph.

Beta is the act of running it against real systems. Nine Warrants
(OW-WAR-0041–0049) are authored against the SAS's own phase-exit criteria and
none is discharged. Two limits are worth knowing before you evaluate anything
here:

**Nothing has run against a real neighbour.** Katana, BLUT, Knowledge Fabric and
Liminal each have a typed, tested adapter on this side and have never been
spoken to.

**Most rules are not reachable from the binary.** Measured 2026-08-20: of twenty
types implementing §40's epistemic classes, §46 independence, §56 resolution and
§44.6 receipts, **twenty were referenced by no code in `war` or the compiler.**
They are implemented and unit-tested; `war check` did not call them.

**Three are now wired**, as of 2026-08-21. `EvidenceOrigin` and `Admissibility`
are read from `- **origin:**` and `- **admissibility:**` bullets and enforced at
obligation parse time, so §40.7's first prohibited substitution — a performer's
own report admitted as independent evidence — is a corpus rule reached by the
shipped binary, with a plant proving it. `GateReceipt` is minted by
`war gate --run`: a completed run now writes a §44.6 receipt with all eighteen
required records and a digest that seals them, and a run that was never askable
writes none.

**Seventeen remain.** OW-WAR-0046 owns the rest.

Pre-1.0 and the protocol is **not stable**: the canonical JSON shape, the digest
domains, and the manifest schema may change in any 0.x release. There is no
allocated enterprise identifier yet (SAS §101.5).

## What works

```bash
war init --namespace OW      # initialize a repository
war new "Ship the thing"     # create a draft Warrant
war check                    # validate, deterministically, with no agent
war check --generated        # also detect drift in committed projections
war compile                  # write the Markdown parent and canonical JSON
```

`war check` over this repository's own Warrants:

```text
PASS manifest.valid                     OW-WAR-0003: manifest and composition are well-formed
PASS assurance.adequacy-review          OW-WAR-0003: controlled assurance carries an adequacy review
PASS relations.parent-digest            OW-WAR-0003: parent war://01a018db-…  contract digest matches
PASS composition.acyclic                parent graph is acyclic across 1 Warrant(s)

4 pass · 0 warn · 0 unknown · 0 error   (worst: PASS)

NOT CHECKED:
  · gate execution — `war gate --run` runs a registered gate, but `war check`
    does not invoke it, so nothing here is evidence a Warrant's gates were run
  · Preflight readiness (§32.7) — 'well-formed' is a claim about the record only
  · bound-atom resolution — `ref =` atoms cannot be fetched offline
  · Source Holder ambiguity and classification propagation (§91.2 tests 14, 15)
  · generated-view drift — pass --generated to compare committed projections

WELL-FORMED (record only — Preflight and gate execution are not implemented)
```

Two things in that output are deliberate and worth noticing.

**The verdict is never the bare word "READY."** §32 defines readiness as
including Preflight, which does not exist yet, so the checker says what it
actually established and names the exclusion inline.

**The NOT CHECKED block prints on every run, including a clean one.** A report
that answers "ok" while whole classes of check go unasked reads as full coverage.

## How a Warrant is stored

Authored atoms are the editable sources. The parent is a projection and is never
edited.

```text
docs/warrants/OW-WAR-0003/
├── manifest.toml            composition and relations (SAS §61)
├── atoms/
│   ├── 10-intent.md         what problem, what outcome, what is out of scope
│   ├── 20-basis.md          governing sources, prerequisites, unknowns
│   ├── 40-work-order.md     deliverables, frozen surfaces, autonomy, rollback
│   ├── 45-milestones.yaml   acceptance checkpoints and dispatchable stages
│   └── 60-assurance.md      obligations, adequacy review, residual risk
└── generated/
    ├── WAR.md               the human parent — do not edit
    └── WAR.json             RFC 8785 canonical JSON
```

Every generated file opens with its provenance:

```markdown
<!--
GENERATED BY OPENWARRANT. DO NOT EDIT.
WAR: OW-WAR-0003
Compilation basis: sha256:7660c2f0edab420e…
Contract revision: 1
Source manifest: docs/warrants/OW-WAR-0003/manifest.toml
-->
```

`war check --generated` compares committed projections against a fresh
compilation and fails on any difference, so a hand-edited parent cannot quietly
diverge from what its sources say.

## Governing specification

`docs/sas/WAR_Software_Architecture_Specification.md`, v0.1.0-draft.1,
sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.

The copy here is byte-identical to the drafted document. Section references
throughout the source (`§65.2`, `RQ-014`, …) cite it.

## Building

```bash
cargo build --workspace
cargo xtask gate
```

`cargo xtask gate` is the aggregate gate (SAS §92): `fmt`, `clippy`, tests,
licenses, and a planted-violation battery. It exits zero only when every positive
fixture passes **and every planted violation is rejected by its intended
control** — the battery deliberately mutates the working tree, confirms the right
rule fires for the right reason, and restores it.

That second half is the point. A validator that returns `Ok(())` unconditionally
passes every positive test ever written.

## Layout

```text
crates/openwarrant-core/       domain types and validators, no I/O   (SAS §79.1)
crates/openwarrant-compiler/   manifest → IR → canonicalize → render (SAS §79.2)
crates/openwarrant-agent/      planning protocol surface only        (SAS §79.3)
crates/openwarrant-cli/        the `war` binary                      (SAS §79.4)
conformance/                   RFC 8785 vectors and the plant battery
docs/adr/                      architecture decisions
docs/sas/                      the governing specification
docs/warrants/                 this repository's own Warrants
```

## What exists, and what has never run

This section was stale in both directions until beta opened, so it now separates
three things a reader would otherwise conflate.

**Implemented and reachable from the binary.** The compiler, `war check`,
`war compile`, all nine §17.5 projections (`war show`), `war diff`, `war plan`'s
two-way agent seam, the Gate Registry, and gate execution — `war gate --run`
really runs a gate and reports §44's askability, execution status and verdict
separately.

**Implemented, tested, and NOT reachable from the binary.** §40's epistemic
classes and their six prohibited substitutions, §46 independence, §56.1's
thirteen resolution requirements, §44.6 receipts, §32 Preflight, §33 context
manifests, §66 the local journal, §67 the Knowledge Fabric action envelope, §68
portable export. These are real code with real tests that no `war` command calls.
OW-WAR-0046 wires them in.

**Not implemented at all.** Preflight execution, and any live conversation with
Katana, BLUT, Knowledge Fabric or Liminal.

See [`CHANGELOG.md`](CHANGELOG.md) and
[`docs/roadmap/PRODUCTION_ROADMAP.md`](docs/roadmap/PRODUCTION_ROADMAP.md).

## About the LamQuant figures

Several Warrants cite measurements from LamQuant, the project OpenWarrant was
built for and is meant to succeed — "of 94 declared gates, 23 invoke a tool,
script, or crate that is not in the tree", and similar. They are quoted rather
than summarised because they are the *evidence* for why particular controls here
exist. A control justified by "gates can be wrong in principle" is a preference;
one justified by a count is a response to something that happened.

Three things about those numbers:

**They are a single measurement, at one commit, on one day.** LamQuant
`5369da81`, 2026-08-17. They were never a running metric.

**They are already stale, provably.** That measurement recorded 167 ADRs. Three
days later the same corpus held 173. The figures describe a corpus at a moment
and should not be read as a current defect count.

**LamQuant is being repaired against exactly these findings.** That is the point
of the exercise — the audit that produced these numbers is what OpenWarrant was
commissioned to make unnecessary, and the repairs are in progress. Citing a
project's own worst measurement is not a criticism of it; it is the reason the
tool exists, and no honest case for OpenWarrant can be made without it.

If you want the current state of LamQuant, measure LamQuant. Do not use these.

## License

**AGPL-3.0-or-later** today; **Apache-2.0** intended when this goes public.

That intent constrains the code now rather than later: every dependency must be
MIT and/or Apache-2.0, enforced by `cargo deny check licenses` inside the gate,
because a copyleft dependency adopted today could not be relicensed afterwards.
See [`RELICENSING.md`](RELICENSING.md) for the preconditions and the exact steps,
and [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a pull request.
