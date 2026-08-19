---
schema: oh.war/atom/v1
adr_uuid: 01a01927-8855-7046-9876-ef13ae754180
local_alias: OW-ADR-0001
role: adr
jurisdiction: authored
order: 30
classification: internal
status: accepted
decided: 2026-08-19
governs:
  - "war://01a018db-19fc-72ba-87b3-c1bd1aec86a8"
---

# ADR OW-0001: Adopt `serde_jcs` as the RFC 8785 canonical JSON implementation

## Status

`Accepted 2026-08-19`

Governs OW-WAR-0003. Required by that Warrant's OBL-000, which cannot resolve
until this decision exists.

## Context

SAS §65.2 requires canonical JSON to be RFC 8785, and §80 requires "an audited
RFC 8785 implementation" without naming one. This choice fixes the exact bytes
that every OpenWarrant cross-system digest is computed over. A defect in it does
not produce a bug that gets fixed later; it silently invalidates every digest
minted before the fix, including any already exported or signed.

RFC 8785 is deceptively hard in two specific places, and an implementation can
look correct while being wrong in either:

1. **Number serialization** must follow ECMAScript `Number.prototype.toString`,
   not Rust's shortest round-trip. The two agree on most values and disagree at
   the boundaries — `1e21`, `1e-7`, `5e-324` — so a wrong implementation passes
   casual testing.
2. **Key ordering** is by **UTF-16 code unit**, not by UTF-8 byte order. These
   agree throughout the Basic Multilingual Plane and diverge above it. A naive
   byte-wise sort orders `"￿"` before an emoji; RFC 8785 orders the emoji
   first, because its high surrogate `0xD83D` is below `0xFFFF`.

## Decision

Adopt **`serde_jcs` 0.2**.

## Rationale

Both realistic candidates were evaluated **empirically**, against the official
`cyberphone/json-canonicalization` test vectors and the ES6 number boundary
cases, rather than on their descriptions:

| | `serde_jcs` 0.2.0 | `serde_json_canonicalizer` 0.3.2 |
|---|---|---|
| official vectors (6 files) | 6/6 | 6/6 |
| ES6 number cases (17) | 17/17 | 17/17 |
| key ordering above the BMP | correct | correct |
| license | **MIT OR Apache-2.0** | MIT |
| `ryu-js` | 0.2.2 | 1.0.3 |
| declared MSRV | 1.85 | none |
| last updated | 2026-03-25 | 2026-02-03 |
| downloads | 2.5M | 5.1M |

Both produced **byte-identical output on all 23 cases**, including the two hard
ones. Both depend on `ryu-js` rather than `ryu`, which is the correct choice and
the single strongest signal that each author understood the ECMAScript
requirement.

Correctness being a tie, the decision falls to licensing and maintenance:

- **Dual MIT OR Apache-2.0.** OpenWarrant intends to relicense to Apache-2.0
  when public. Either candidate permits that — a permissive dependency does not
  constrain our licensing — but taking the Apache-2.0 branch of a dual-licensed
  dependency carries an explicit patent grant that MIT does not offer. For a
  protocol implementation others are meant to interoperate with, that is worth
  more than it would be for an internal utility.
- **A declared MSRV** is evidence the maintainer tracks compatibility
  deliberately rather than incidentally.

**The stakes of this decision are lower than they appear, and deliberately so.**
Our conformance suite tests canonical *output* against external vectors, not
against this library's behaviour. That means the library is replaceable: a
regression in `serde_jcs`, or a future need to move to
`serde_json_canonicalizer`, is caught by our own tests and costs a dependency
swap rather than a wire-format migration. The architecture is what makes the
choice reversible — so the right move was to decide on evidence and move, not to
deliberate further.

## Alternatives Considered

- **`serde_json_canonicalizer` 0.3.2** — equally correct on the evaluated
  corpus, twice the adoption, and on a newer `ryu-js`. Rejected only on the
  licensing margin above. It is the designated fallback, and swapping to it
  should be a single-line change plus a green conformance run.
- **Hand-rolling RFC 8785** — rejected. The ES6 number path requires a correct
  shortest-round-trip float formatter with ECMAScript exponent rules, which is
  precisely the wheel `ryu-js` exists to avoid reinventing, and getting it
  subtly wrong is the failure mode with the worst blast radius in this system.
- **Deferring canonicalization and digesting bytes directly** — rejected.
  Digests over non-canonical bytes are stable only for one serializer, which
  makes cross-system verification impossible and defeats §65 entirely.

## Consequences

**Good.** OW-WAR-0003 unblocks. Canonicalization has an empirical basis, and the
evaluation harness that produced it is reusable as the seed of the §91.1
conformance suite.

**Bad.** `serde_jcs` pulls `ryu-js` 0.2.2 while the alternative pulls 1.0.3. If
the 1.x line contains number-formatting corrections not backported, our output
could diverge from a peer implementation using the newer line. Mitigated, not
eliminated: our conformance suite re-runs the ES6 boundary cases on every gate
run, so a divergence surfaces as a test failure rather than as a bad digest. It
is listed as residual risk in OW-WAR-0003.

**Unchanged.** `cargo deny check licenses` continues to gate the dependency
graph. `ryu-js` is `Apache-2.0 OR BSL-1.0`; the Apache-2.0 branch is already on
the allowlist, so no allowlist change is required to adopt this.

## Validation

Watch for: any conformance failure on the ES6 number cases after a dependency
update, which means the library moved under us; and any peer implementation
reporting a digest mismatch on identical input, which is the symptom the
`ryu-js` version gap would produce.
