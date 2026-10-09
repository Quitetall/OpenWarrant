# Current-basis runtime receipt assessment

Candidate SDK implementation, 2026-10-09. This is performer-reported work, not independent verification, participant agreement or human acceptance.

`openwarrant_compiler::runtime_basis::assess_runtime_basis` checks the whole captured Compilation Basis. It checks manifest/source membership, requires a valid required milestone source, parses every milestone source strictly, and refuses duplicate stage or milestone identities across sources. A valid inventory with no Katana/BLUT stages has no runtime receipt requirement. Missing or malformed inventory cannot establish that exemption.

For each Katana/BLUT stage, the caller supplies exactly one selected recorded attempt. Missing receipts remain UNKNOWN. Multiple selected attempts and unrelated receipts are refused; historical attempts remain in the caller's store. The SDK checks current Warrant UUID, contract digest, workspace digest, milestone membership, normative omissions and provider kind. It recomputes the existing canonical Dispatch digest with the digest field empty, using the existing domain. Native receipt matching then uses the existing provider-verifier interface. Refusal takes precedence over UNKNOWN, which takes precedence over matching; all inventoried stages retain their own observations. Unknown cost remains visible and cannot satisfy a mandatory hard spend cap.

## Trust boundary and remaining scope

The caller must authenticate source capture, recorded-attempt provenance, attempt selection, provider identity/backend mapping and dispatch policy. An in-memory constructed dispatch is not proof of a recorded execution. The provider verifies native seals and execution observations. The SDK neither invents native wire protocols nor discovers, captures or persists receipts. These tests use explicitly synthetic adapters and establish no live provider execution.

The legacy resolver is unchanged. Durable attributable imports, current-attempt selection from the store, real provider interfaces, resolver integration, independent verification and participant acceptance remain incomplete. OW-WAR-0149 remains in progress.

## Checks

Pinned Rust 1.97.1. Public integration controls in `crates/openwarrant-compiler/tests/runtime_basis.rs` cover two-stage completeness, current contract and dispatch binding, duplicate attempts, unrelated receipts, missing/malformed source inventory, duplicate stage identities, wrong provider kind, unavailable verification, failed execution and mandatory spend accounting. The build and full repository gate are recorded with this batch separately; passing synthetic controls is not provider qualification.
