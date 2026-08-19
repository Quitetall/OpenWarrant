---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-72ba-87b3-c1bd1aec86a8
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

- SAS §14 (Workspace Basis), §63 (canonical WAR IR), §64 (format basis),
  §65 (digest domains), §69 (protocol versioning), §91.1 (canonicalization
  conformance).
- RFC 8785, JSON Canonicalization Scheme.
- Parent Warrant OW-WAR-0001, contract revision 1.

## Prerequisites

- OW-WAR-0002 resolved: there is something to lower.
- **An implementation ADR selecting the RFC 8785 library, resolved before any
  code under this Warrant is written.**

## Assumptions and Unknowns

- **Blocking unknown — the only true blocker in Phase 1.** Which RFC 8785
  implementation to adopt. §80 requires "an audited RFC 8785 implementation" and
  names none. Candidates include `serde_jcs` and `json-canonicalization`. The
  choice must clear two independent bars: correctness against the RFC's own test
  vectors, and a license compatible with this repository's intended Apache-2.0
  future. A crate that fails the second is unusable regardless of how good it is.
- **Evidenced premise.** SHA-256 over bytes is already correct here, pinned in
  `openwarrant-compiler::digest` against published vectors for the empty string
  and `"abc"` — external vectors, not values read back out of the
  implementation.
- **Accepted residual risk.** The IR shape of §63 has twelve top-level sections
  and Phase 1 populates perhaps five. The unpopulated sections are structural
  placeholders. An empty section and an absent section must serialise
  differently, or a later phase that starts populating one will silently change
  every digest minted before it.

## Constraints and Invariants

- **No placeholder canonicalizer, at any point, even temporarily.** A digest
  minted under a stand-in is indistinguishable from a real one after the fact.
  If the library decision is not made, this Warrant does not start.
- **Domain separation is mandatory and structural.** Every digest is computed
  over `{"digest_domain": "<uri>", "payload": …}` (§65.2). A digest computed
  over a bare payload is a defect even when it is unique in practice.
- **Generated Markdown must not affect the contract digest** (§91.1 test 3).
  Rendering is a projection; changing how a document reads cannot change what
  was authorized.
- **Unknown required fields fail closed; unknown optional namespaced extensions
  survive a round trip** (§91.1 tests 4 and 5, §69.4).
