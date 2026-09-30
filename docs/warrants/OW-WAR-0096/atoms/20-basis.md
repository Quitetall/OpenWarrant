---
schema: oh.war/atom/v1
warrant_uuid: 01a0ad2f-c0fd-7471-a00e-c7a9eb8f89d1
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

Owner approved the model: agents may draft changes; only transitions authorized
under previous trusted authority become effective. Owner requested implementation.
Candidate sources: docs/sas/drafts/1.0.0-rc.3/sdk-contract.md authority section,
authority-transition-plan.md and authority-transition.adr.md. Current source base:
ecf57e70b213f650315c23dbb4d875bddfa22768.

Trusted deployment requires separate protection for keys, store, verifier and
activation. Current unrestricted coding session cannot prove human presence or
establish its own trusted bootstrap. These deployment limits do not block writing
and testing the candidate SDK/CLI against disposable keys and state.
