# OW23: portable context and capability refusal

Source: `b6846f7f8e077b794b16102f1a63cc1e93000006`. Requested transport and capability-membership slice delivered,
**unverified**. Full OW-WAR-0023 remains in progress. No legacy signature,
assurance disposition or resolution changed.

`war dispatch-bundle create` captures exact selected bytes and support references
for an existing Dispatch. `check` reads that capture offline against an externally
supplied bundle digest. Required and selected optional atoms match contract source
digests; section views retain exact provenance. A capability is denied unless it
is explicitly listed in the bound policy and the caller supplies that policy's
independently established digest. No host permissions are inherited.

[Command and SDK reference](../../../cli/dispatch-bundle.md) explains candidate
formats, attachment mapping, resource limits and trust responsibilities. Existing
pinned Dispatch schema and compilation remain unchanged. Packaging adds no model
calls, semantic compiler, execution or signing path.

## Evidence

- Eight public CLI/SDK tests passed on Rust 1.97.1. These compile real synthetic
  Dispatches, move/capture their context, remove the copied repository and read
  exact bytes offline. Controls cover default denial, explicit-policy positive
  and negative cases, omitted/changed bytes, selected optional source rebinding,
  section provenance, reserved-reference collisions, contradictory IR metadata,
  deep typed input, required attachments, deterministic bytes and no overwrite.
- Clippy passed for all CLI targets on Rust 1.97.1.
- Full local gate passed 14 steps and 308 controls **before the final IR metadata
  consistency fix**. The log name preserves this boundary. Final source must
  pass the protected-main PR's full gate before merge.
- Separate Spec and Standards reviews passed after confirmed fixes. Actual
  free-local LAMU source review returned PASS WITH NITS; findings were checked.
  These reviews are development evidence, not formal Warrant verification.

The independent reviews found real optional-atom binding, reference-shadowing
and recursive-input defects. A further local probe found contradictory captured
IR metadata. Regression controls now refuse each case. The frozen compiler's
existing actor-projection comparison is also exercised with a permitted human
view and a forbidden contract change.

## Remaining scope

The transport proves captured-byte correspondence, not complete semantic
selection or classification-policy enforcement under legacy SAS §47.2. Those
broader OW23 requirements need provider/workflow evidence or explicit scope
reconciliation. Runtime capability enforcement belongs to the harness; this SDK
supplies the pre-action decision only. Authority, policy revocation, verified
start and human acceptance cannot be inferred from a caller-provided digest.
Candidate format adoption and formal assurance remain separate. Stable 1.0 is
not published by this work.
