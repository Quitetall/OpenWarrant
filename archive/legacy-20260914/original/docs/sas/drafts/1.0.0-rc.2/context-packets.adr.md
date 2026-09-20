# Architecture proposal: standalone RC.2 documents, context, and assurance

Status: proposed, unregistered, unaccepted. Local draft name:
`rc2-standard-and-compiler`. No official ADR allocation or signature is claimed.
Target: [SAS 1.0.0-rc.2](WAR_Software_Architecture_Specification.md) and its normative
companions. This replaces the earlier unaccepted context-only proposal.

## Problem

The inherited draft mixed an atom-based governed workflow with a lightweight
portable document standard. It also required sibling runtimes, froze live delivered
paths, and treated several workflow controls as universal authoring prerequisites.
The owner selected standalone compilation, low-friction prototypes, exact binding
context, and optional human-backed qualification. Patching isolated sentences left
contradictory requirements; RC.2 consolidates one coherent contract.

## Proposed decisions

1. Separate document validity, compiler conformance, optional assurance, and later
   workflow readiness. A valid minimal draft is useful before approval or fixtures.
2. Author one Markdown file with TOML metadata, stable explicit units and simple
   conditional pointers. The TOML dependency already exists; legacy restricted
   YAML readers keep their contracts. New syntax is a separate versioned adapter.
3. Resolve paths to immutable source snapshots automatically. Assemble an explicit
   source set, then select exact units and dependencies without a model call.
   Missing condition input conservatively includes required content with uncertainty.
4. Use a limited condition table: stage/subsystem/path, OR within each field and
   AND across fields. No arbitrary expression language in v1. Include consistent
   context cycles once and report them; reject unresolved/conflicting meaning.
5. Deliver a directory packet with a clear entry, canonical machine view, manifest,
   and exact source blobs. Offline required context is mandatory; resolver optional.
   Entry context size and portable package size are different measurements.
6. Permit policy-driven autonomous work and delegated SAS adoption, while preserving
   human-only effective-policy edits, signed Warrant completion and assurance.
   A final-result baseline permits later qualification with truthful fixture history.
7. Preserve accepted historic bytes and allow explicit successor work at the same
   pathname. Migration must recover those bytes before changing live-pin behavior.
8. Build eleven bounded library features first, then CLI/compiler composition and
   conformance. Knowledge Fabric Compiler and full workflows follow Stable.

## Alternatives and costs

- Keeping five mandatory atoms would reuse more existing machinery but violate
  the selected normal single-Markdown authoring format. Keep atoms as legacy input.
- Full YAML or JSON headers are possible; TOML gives typed arrays/tables with an
  existing dependency and no new hand-written nested YAML subset. This changes
  source syntax, so it is explicit here and does not modify OW-ADR-0002/0003.
- Heading-only references break on rename and can be ambiguous. Marker IDs cost
  one line per unit but preserve stable targeting and exact source spans.
- General expressions offer more routing power but enlarge validation and security
  scope. The selected condition grammar covers the agreed v1 use cases.
- Rejecting every text dependency cycle creates avoidable friction. Closed consistent
  sets can be read safely; execution DAG cycles still block scheduling.
- Resolver-only packets fail offline delivery. Full source blobs cost transfer/storage
  bytes and may force authors to split access-controlled sources; they preserve
  independently checkable byte provenance. Smaller entry projections still reduce
  initially loaded agent context. No size/speed improvement is claimed without proof.
- Automatic marks would remove review friction but contradict the chosen human-backed
  claim. Unmarked finished work is a valid resting point without a review queue.

## Compatibility and adoption

Preserve OW-ADR-0001 canonicalization, old schemas/digest domains and signed subjects.
OW-ADR-0012 continues to govern current corrections until adopted migration.
OW-ADR-0019's proposed 1.1 batch/rendering edition is not adopted by this proposal:
retain exact batch subjects and invalidate changed requests, but use the owner's
later authenticated-session approval preference for the future workflow boundary.
No timeless permission window or renderer-generated authority is introduced.

RC.1's historical accepted label remains `1.0.0` in its original records. The owner
now names that edition rc.1 and this candidate rc.2. Adopting the candidate requires
its exact source-set manifest and the existing authority process; this file only
makes the change concrete for review. No production code changes are part of the
specification task.

## Evidence required

Run reference-example checks now, then implement the F01–F11 and T01–T56 matrix.
Independent review checks consistency; source/example audits cannot substitute
for the production conformance suite. Phase 2 release proof and later workflow
proof have separate gates in the build scope. No already-complete claim is made.
