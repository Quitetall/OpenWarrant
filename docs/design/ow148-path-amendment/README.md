# OW-WAR-0148: proposed path amendment

## Current repair route: restore the signed paths

The implementation repair moves the three real files to the existing signed
paths instead. Their bytes remain unchanged, and Rust's `states` module keeps
its public name through `#[path = "state_cmd.rs"]`. No wrapper or empty file
stands in for an implementation. The authorization, delivery declarations,
work order and assurance scopes remain unchanged; Git records the renames.
Fresh compilation and the relevant checks are still required before merge.

The analysis and patch below are retained as an **unused alternative**. They
describe the earlier inspected commit, not a pending authority act. Do not
apply the patch as part of the restoration. An amendment is required only if
the delivery declarations are changed; restoring the implementations to the
already authorized paths does not make that change.

**Unused draft only.** Revision 1 remains authoritative. The retained alternative
proposes a revision; it does not authorize one, qualify the implementations or resolve the
Warrant. No signed record, active atom or delivery declaration was changed.

## What the alternative amendment would change

Three required deliverables name planned paths that do not exist. The actual
implementations are committed under these names:

| Deliverable | Signed path | Proposed path |
|---|---|---|
| D-041 | `crates/openwarrant-cli/src/state_cmd.rs` | `crates/openwarrant-cli/src/states.rs` |
| D-051 | `conformance/plants.d/76-ticket-features.sh` | `conformance/plants.d/77-ticket-features.sh` |
| D-052 | `conformance/plants.d/77-typed-demo.sh` | `conformance/plants.d/78-typed-demo.sh` |

`states.rs` identifies itself as M4's implementation; the CLI dispatches
`Command::State` to `states::enter`. The two plant files identify themselves as
M5 and exercise ticket features and the password-reset demonstration. These
are existing implementations, not proposed stubs or renamed copies.

The [proposed patch](proposed.patch) updates the three declarations and the
same names in the work order and assurance scopes. It preserves deliverable
IDs, required flags, obligation IDs, checks and behavior. It does not edit
`authorization.toml`, an existing signature or a generated projection.

The [observations](observations.json) identify the inspected commit, retained
authorization bytes, current authored sources, proposed source bytes and
actual implementation bytes. These are SHA-256 observations for this review,
not a new protocol digest domain or proof of independent verification.

## Why changing the declarations would need an amendment

The delivery set is part of authorization. Changing its paths changes what the
authorization covers, even when the intended implementation is the same.
The work order also requires escalation for a moved contract digest. This is
a proposed manual revision under SAS §31, not an automatic typo correction.
The old revision and signature must remain traceable through the supported
revision and authority history.

`war prepare --all --dry-run` reported `deliver.missing` for these three paths.
That finding does not prove that all other OW-WAR-0148 requirements are met.
Do not insert empty files, weaken missing-delivery checks or claim the Warrant
complete to remove the finding.

## Apply only through a reviewed revision

1. Check the source and implementation hashes against `observations.json`.
   Re-review any changes. `git apply --check` proves patch applicability only.
2. Prepare a fresh amendment with `war amend OW-WAR-0148`. Record the path
   changes and corrected references, why they are needed, affected M4/M5
   stages, artifact admissibility and the required preflight/evidence reruns.
   Do not invent an authorizer, effective approval time or approval act.
3. Stage the proposed source revision in a dedicated checkout using the patch.
   Preserve revision 1 and its authority history. Run `war check OW-WAR-0148`
   and emit `war authorize OW-WAR-0148` for review. A changed delivery set and
   contract require new authorization; an old signature is insufficient.
4. Before asking the human to sign, run
   `war sign OW-WAR-0148 --dry-run`. Resolve each named refusal first. Only
   the authorized human can perform the actual signature.
5. After the amendment is authorized, regenerate projections with
   `war compile`; check them; then re-run preparation and the required checks
   and independent verification on the new revision. Preserve previous evidence
   as history, without treating it as proof of the changed reviewed subject.

## Observed checks

At the recorded commit, the three old paths are absent and the three proposed
paths are regular files. The patch passes `git apply --check` and changes only
the three authored source files listed in the observations.

`war sign OW-WAR-0148 --dry-run` reports `sign.nothing-pending` on the unchanged
corpus. This is expected: the proposal is not applied and no new authorization
request is pending. It is **not** a successful signing preflight or permission
to sign this proposal. No model, independent verdict or human act was recorded.
