---
schema: oh.war/atom/v1
warrant_uuid: 01a06a12-0aa2-7503-b589-67cf75905be4
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

This repository can deliver an artifact and it can refuse a change to one. It
cannot repair one.

`deliverables.toml` pins the bytes of each delivered file, and `war check` raises
`deliverable.digest-drift` when those bytes move. Once the Warrant is resolved,
that pin has no authorized way to change. Three defects hit that wall on
2026-09-03, all found by other projects reading our seams:

- the Dispatch compiler drops every continuation line of a wrapped instruction,
  so a stateless actor is told less than the contract says
  (`crates/openwarrant-compiler/src/dispatch.rs`, deliverable D-002 of resolved
  OW-WAR-0056);
- `war status` falls back to the literal namespace `"OW"` instead of the
  repository's configured one, so another program's phases render under this
  program's name (`crates/openwarrant-cli/src/status.rs`, deliverable of
  resolved OW-WAR-0055 and OW-WAR-0057);
- neither `war sas accept` nor `war authorize` validates `effective_time`, so a
  placeholder can enter an immutable record
  (`crates/openwarrant-cli/src/sas.rs`, deliverable of resolved OW-WAR-0058).

Each fix is small. None can be made. The gate is not wrong to refuse — a
delivered artifact must not move because the performer noticed something
afterwards. What is missing is the act that lets it move for a reason.

The failure mode is not the defects. It is that known-wrong artifacts accumulate
until somebody decides the digest gate is an obstacle rather than a control, and
routes around it. That decision, once taken, is taken for every artifact.

## Desired Outcome

A named, authorized act supersedes a delivered artifact of a resolved Warrant,
leaving the original record intact and legible as history — with the superseding
act refused unless it carries what makes it answerable.

## Scope

The correction act; what it records; what refuses it; §34.4's shape applied to
deliverables rather than requirements.

## Non-goals

- **No weakening of `deliverable.digest-drift`.** Drift with no correction record
  must stay an error. A repair path that silences the detector has removed the
  control it was meant to complete.
- **No annulment of resolutions.** §56.2 records stay immutable and stay
  satisfied. A corrected artifact does not unmake the judgment that accepted it.
- No fixing of the three defects here. They are the evidence that the act is
  needed, and each is its own work once the act exists.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-020` — Partial; the correction act is a normative decision and
  gets an ADR.
- `WAR-SAS-RQ-084` — Partial; the superseded deliverable record remains
  available.
