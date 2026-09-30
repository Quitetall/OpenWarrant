---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1eb7-78d3-8eb0-45b5463010df
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

- `conformance/rc2/`
- `docs/standard/1.0.0/`: the owner-approved Stable SAS/format/schema/fixture source set, distinct from the retained RC.2 candidate
- `crates/openwarrant-core/src/document.rs` and versioned schema constants: only the approved Stable edition identifiers and explicit compatibility mapping
- `docs/releases/1.0.0/qualification.md`
- `docs/releases/1.0.0/source-set.json`
- `docs/releases/1.0.0/context-benchmark.json`
- `docs/sas/revisions/<owner-approved-stable-record>.toml`
- This Warrant's evidence/ directory: immutable command outputs, fixture/build
  identities, independent review request and response references.

Paths are planned ownership boundaries, not claims that these files already
exist. Angle-bracket record names require the authorized owner's allocation;
they are not identifiers allocated by this draft. Shared driver/module exports
may change only for this slice. Reconcile shared edits before integration.

## Stable edition promotion

Prepare the exact Stable source set before final qualification. Review its
diff from the adopted RC.2 candidate, including final schema identifiers,
compatibility aliases and acceptance-record identity. Preserve the RC.2
source set and the historical record literally labeled 1.0.0. Never overwrite
either to obtain the Stable spelling. The owner-approved adoption procedure
must distinguish these editions. Substantive format or required-expectation
changes require approved revision and rerun affected implementation checks.
The final conformance run exercises the actual Stable identifiers and bytes
that will ship, not only the earlier RC.2 examples.

## Frozen Surfaces

Previously authorized contract revisions, original accepted source bytes,
signatures and evidence remain immutable. Legacy parsers, digest domains and
command meanings remain compatible unless an approved migration says otherwise.
Required behavior, permission boundaries and acceptance expectations are binding;
internal implementation steps can adapt within those bounds.

## Premade Instructions

Use the seam and complete case inventory in Basis. Prepare/reuse acceptance
fixtures before implementing the corresponding behavior. Use one isolated Git
worktree per implementation Warrant and serialize writers within it. Reviewers
use a separate context/workspace and cannot clear the performer's gate by
repeating its assertions. A stage may be split into smaller reviewable changes
without expanding scope; implement one positive/refusal slice at a time.

Run all T01-T56 through integrated public paths, exact JSON/rendering goldens, resource/write-failure cases, offline Linux/macOS consumers and cargo xtask gate. At least twelve fixed context tasks meet 100% expected required-unit coverage and zero binding-byte changes or invented authority; include at least three no-SHALL dependency tasks, three UNKNOWN tasks and three expected refusals. Independent verification and human acceptance bind the exact Stable candidate.

Missing cases, false-green expected-refusal handling, changed release bytes after review, unobserved platform checks or incomplete context prevent qualification. Optional paid agent trials are required only for any claimed agent-efficiency improvement, not for deterministic compilation.

## Autonomy and Escalation

Agents may draft, implement within authorization, run checks and report evidence.
They cannot authorize, sign, accept a SAS, clear their own independent gate or
resolve this Warrant. Escalate changes to wire meaning, digest domains, authority,
required expectations, scope or release naming. Default repair limit is three
cycles unless the owner chooses another; actual repository spend/time limits
must be explicit before paid or autonomous execution.

## Rollback

Preserve the pre-change worktree and accepted source basis. Failed operations
leave prior output intact. Revert an unaccepted implementation branch or issue
an approved successor/correction; never erase or regenerate signed history.
