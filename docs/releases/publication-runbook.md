# Stable publication runbook — not ready to execute

OW91 depends on OW90's exact four-phase qualification and owner release permission.
Both remain unestablished. The owner also requires every remaining Warrant to be
completed or explicitly reconciled before release; see the
[reconciliation queue](warrant-reconciliation.md). The workflow deliberately refuses all tag publication.
Do not remove that refusal or push a release tag to test this runbook.

## Prepare the exact release subject

1. Finish the missing work in [remaining build scope](remaining-build-scope.md).
   Replace the incomplete inventory with a new, independently reviewed release
   manifest. Preserve the old inventory and its source identities.
2. Freeze the accepted SAS source set, standard schemas, compatibility fixtures,
   SDK/API versions, lockfile, supported platform profiles and release notes.
   Bind native build receipts to the exact source tree and artifacts. The present
   caller-supplied binary linkage is insufficient.
3. Collect all four phase exits, including real workflow/adapters, actual-user
   observations and hardening results. A passing repository gate alone does not
   qualify the product. Human assurance records bind their exact result subjects.
4. Freeze the full Warrant inventory. Every outstanding scope must have completion
   evidence or an explicit accepted reconciliation route; preserve original records.
   Missing records, undecided dispositions and unresolved integrity issues refuse
   release readiness. This is separate from awarding a Verified mark.
5. Prepare a reviewable publication change: final versions, artifact names and
   digests, destinations, crate dependency order, release notes, intended tag and
   an enforced qualification/permission check replacing the temporary refusal.
   Test negative cases: absent permission, stale source, changed artifact, missing
   platform, incomplete phase and wrong version must not publish.
6. Obtain owner permission for that exact publication. Existing permission to
   implement Warrants does not authorize an irreversible registry release.

## Execute only after those prerequisites

- Build and qualify the final candidate on native Linux and macOS. Independently
  verify all source and binary digests before publication; no rebuilding after
  approval without a new artifact review.
- Publish only the approved tag, release assets and registry versions. Record
  every destination and returned identity. Cargo publishes manifest versions;
  prerelease tag spelling does not change a crate's version. Require the
  approved tag version to match the manifest version before registry publication.
- Re-download each public artifact and registry package into its own fresh, empty
  temporary directory. Refuse reused destinations and incomplete downloads.
  Check exact approved byte digests and registry metadata. Run the approved
  install/examples on the downloaded artifacts. Record failures separately from
  successful publication; a server accepting an upload is not download proof.
- Preserve the public release receipt, native observations and independent review
  with project history. Report partial publication honestly and follow the
  approved recovery plan. Never silently overwrite or republish a used version.
- Emit the configured final release completion word only after publication and
  re-download verification are established for every required destination.

## Current result

No tag, registry version or public release was created by this work. No permission
request is ready: missing product and qualification evidence comes first. The
existing release workflow remains blocked rather than representing a rehearsal
as Stable 1.0.
