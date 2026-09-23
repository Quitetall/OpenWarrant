---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f41-7a13-9d41-14fd18657244
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §11 (component ownership): KF owns authorization, lifecycle, role
  authority, judgments and resolution. §11.1: registering does not transfer
  source authority. §11.2: once registered, KF owns lifecycle and
  controlled actions.
- SAS §12.4: no claim of globally authorized or effective state before
  registration. §12.6: offline creation keeps working.
- SAS §67 (the action vocabulary, the §67.1 envelope, §67.2 server time) and
  §72.4 (the CLI SHALL not bypass KF authority).
- SAS §27.2: authorize, resolve, SAS acceptance and correction are human
  acts.
- RQ-060 (this Warrant), RQ-076 (KF commands use typed actions), RQ-020
  (a normative decision is an ADR), RQ-070 (offline drafts).
- OW-ADR-0005 (local gates are candidates), OW-ADR-0012 (the correction
  act), OW-ADR-0019 (the batch act), OW-ADR-0021 (ownership).
- Code as it is: `crates/openwarrant-cli/src/kf.rs` (`war kf health`,
  `war kf act`), `crates/openwarrant-core/src/seam.rs` (the 32 §67 names),
  `crates/openwarrant-cli/src/authority_check.rs` and `sign.rs` (local
  human acts).
- Knowledge Fabric, read at `/mnt/4tb/openhuman-knowledge-fabric` commit
  `3d3c871e`: `docs/decisions/0019-warrants-as-institutional-record.md`.
  One `warrant` object per OpenWarrant UUID; all 32 actions owned;
  lifecycle phases along §24.7; contract revisions recorded as OpenWarrant
  computed them.
- OW-WAR-0029 and OW-WAR-0044: authorized, not delivered. OW-WAR-0114 named
  this gap (`roadmap://OW-PHASE-4/kf-authority`).

## Assumptions

- A-001: the §67 names in `seam.rs` match what KF accepts. KF ADR 0019 says
  its `warrants` group spells every action "as OpenWarrant's seam spells
  them". Confidence: high, at KF commit `3d3c871e`.
- A-002: a Warrant that is never registered keeps working exactly as today.
  §12.6 and RQ-070 require it. Confidence: high.

## Unknowns

- U-001 (blocking): how the tool knows a Warrant is registered. OW-WAR-0029
  owns the registration record and has not delivered it. The ADR cannot
  name a signal that does not exist.
  *Resolution requirement:* the owner answers Q-001, or OW-WAR-0029
  delivers the record.
- U-002 (blocking): what a local human act does on a registered Warrant.
  This is the decision the ADR records, and it is the owner's.
  *Resolution requirement:* the owner answers Q-002.
- U-003 (blocking): whether a local authorization made before registration
  carries into KF as the authorized revision, or KF must authorize again.
  KF ADR 0019 refuses an authorization that names a foreign digest; that
  fits either answer.
  *Resolution requirement:* the owner and KF's owner answer Q-003.
- U-004 (non-blocking): two local acts have no §67 counterpart: the
  correction act (OW-ADR-0012) and the batch act (§27.6). The ADR must say
  what each does on a registered Warrant. It may say "refused; no KF path
  yet" and name that as a KF gap.

## Questions for the owner

- Q-001 (U-001): how is a Warrant known to be registered?
  (a) a registration receipt file in the Warrant directory, holding KF's
  receipt and the enterprise identifier, checked like a signature;
  (b) a non-empty `enterprise_id` in the manifest, accepted only with such
  a receipt beside it;
  (c) ask KF at each act. This breaks offline use (§12.6).
- Q-002 (U-002): on a registered Warrant, what does `war sign` do for
  authorize or resolve?
  (a) refuse locally and print the §67 action request for a human to post
  with `war kf act`;
  (b) sign locally, then post the matching §67 action with the local
  signature as its evidence;
  (c) keep signing locally and let KF mirror it. This contradicts §11.2 and
  §72.4; listed only for completeness.
- Q-003 (U-003): does a pre-registration local authorization carry over?
  (a) yes, submitted as the authorized revision with its signature as
  evidence;
  (b) no, KF authorizes afresh and the local record stays as history;
  (c) KF decides by organization policy.

## Residual risks

- R-001: KF's lifecycle or action names change before the follow-on
  implementation. The ADR cites KF at a commit, so drift is visible.
- R-002: the ADR is accepted before OW-WAR-0029 delivers, and the signal it
  names differs from what OW-WAR-0029 builds. The follow-on Warrant then
  supersedes this ADR.
