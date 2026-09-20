---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-75b4-9586-0aae240f38bc
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

- SAS §15 (one logical document), §17 (parent document rules), §59.2 (generated
  view policy), §103 (default full Warrant rendering), §91.2 test 16.
- Parent Warrant OW-WAR-0001, contract revision 1.

## Context

This repository sets `[generated] commit = true, verify_drift = true`. That
choice is what makes §17.3 checkable: the compiled parents are in Git, so a
fresh compilation has something to be compared against. Under
`commit = false` the parents would simply not exist between compilations and
there would be nothing to drift.

## Prerequisites

- OW-WAR-0003 resolved. Without canonical IR there is nothing to project from,
  and a renderer built over an uncanonicalized value would produce output that
  differs between runs for reasons unrelated to content.

## Assumptions and Unknowns

- **Evidenced premise.** The five authored Warrants exercise the `delivery`
  profile's required roles, so the renderer meets every section it must render.
- **Accepted residual risk.** §103's rendering includes sections — Execution,
  Resolution, Relations and Integrity — that have no source in Phase 1. §16.1
  says the parent SHALL omit inapplicable optional roles rather than render
  empty ceremonial headings, so they are omitted. The risk is that "omitted
  because inapplicable" and "omitted because the renderer forgot" look identical
  in the output.

## Constraints and Invariants

- **The parent is never authoritative because it is committed** (§17.2). It is a
  reproducible projection, and the drift check is the proof.
- **Every generated file carries the §17.1 header**, naming the WAR, the
  compilation basis digest, the contract revision, and the source manifest.
- **Generated Markdown changes must not move the contract digest** (§91.1
  test 3). Rendering is downstream of meaning.
- **Recompilation on an unchanged tree is a no-op.** If `war compile` rewrites a
  byte — a timestamp, an ordering, a version string — the drift check reports
  drift on every run and is disabled within a week.
