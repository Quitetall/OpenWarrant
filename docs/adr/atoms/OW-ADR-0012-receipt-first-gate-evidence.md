---
schema: oh.war/atom/v1
adr_uuid: 01a03f3d-8f08-7e91-9efc-d6247bfb5aa2
local_alias: OW-ADR-0012
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a021a6-0dfc-7c2c-b61a-06b3aad19004"
---

# ADR OW-0012: Make Gate evidence receipt-first and byte-bound

## Status

Proposed. Requires human acceptance before it governs work or authorizes the
corresponding SAS and wire changes.

## Context

`GateRun` records askability, execution status, and verdict. Those facts alone
do not establish which definition and binding executed, which actor produced
the evidence, which subjects and fixtures were measured, or which stdout and
stderr bytes support the verdict. Treating a standalone passing `GateRun` as
resolution evidence would therefore permit mutable or self-asserted history to
satisfy an obligation.

The alpha `GateReceipt` model was never emitted by the shipped binary. Its
`runner` field named an instrument such as `war gate --run`, not the actor that
produced the evidence, and its output references did not bind output bytes.
The implementation also digested a receipt under the Gate Run domain. Adding
required producer and stream-digest fields changes protocol semantics and must
not be shipped as an invisible edit to the old shape.

## Decision

Recorded Gate evidence is an immutable bundle committed by its receipt. The
receipt is published last. A resolver starts from that receipt and derives the
exact sibling Gate Run, Gate Binding, runner-owned test-selection observation,
stdout, and stderr objects from one canonical run identity. A standalone Gate
Run is history, never admissible resolution evidence.

The first emitted receipt wire is `oh.war/gate-receipt/v1`, with kind
`gate_receipt`. It requires a distinct producer-actor identity in addition to
runner identity, exact SHA-256 digests beside stdout and stderr references, and
an exact reference and SHA-256 digest for a
`oh.war/test-selection-observation/v1` object. The receipt producer SHALL equal
the producer selected by the authorized Gate Binding; that actor SHALL hold an
effective Verifier assignment when execution starts. Caller-supplied producer
labels are not evidence. Unknown non-namespaced fields fail closed. Unversioned
alpha-shaped receipts may be preserved as legacy artifacts but cannot satisfy
a required pass.

Recorded execution is limited to exact runner-owned adapters. An adapter binds
one qualified Gate Definition implementation, output schema, argument vector,
and selected-test manifest. After execution it emits the observed run identity,
definition digest, adapter identity, count, and manifest. Definition prose alone
cannot prove which tests ran. Unknown adapters may still be probed without
recording, but they cannot mint resolution evidence. Raw evidence references are
admissible only when that exact adapter proves how it consumed them.

Receipt digests use the distinct domain `oh.war/gate-receipt/v1`. Gate Run and
Gate Receipt preimages must never share a digest domain. The receipt digest is
computed over the complete receipt with its own digest field empty, so schema,
kind, identities, bindings, inventories, chronology, outputs, and verdict all
participate.

Admission also requires the exact qualified nonmutating Gate Definition and
Binding, exact subject and fixture inventories, exact selected-test manifest,
canonical monotonic timestamps, effective verifier authority for the producer,
and evidence-policy permission for performer-authored output. Unsupported
Binding parameters or pass predicates refuse before execution.

Legacy ADR terminal support carries the selection observation as its own exact
`ContentBinding`. Pinned Resolution evidence includes that path in its assurance
snapshot. Omitting or substituting the observation therefore invalidates the
same bundle before any passing Gate Run can be admitted.

## Consequences

Resolution receives a typed admissibility witness rather than a raw Gate Run.
Tampering with any bundle member invalidates the whole candidate. Multiple runs
remain immutable instead of overwriting one "latest" file.

The wire and SAS changes remain non-governing until a human accepts this ADR.
Acceptance requires updating SAS §§44.6 and 65, implementing the versioned
receipt shape and digest domain, preserving explicit legacy rejection, and
passing positive plus planted-negative controls through the shipped binary.

Because no repository Gate receipt exists from the alpha implementation, this
decision does not rewrite deployed evidence. It still treats the old shape as a
breaking semantic predecessor rather than pretending the change was additive.
