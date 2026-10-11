# OpenWarrant SAS editions

Current product-direction candidate: **[1.0.0-rc.3](1.0.0-rc.3/README.md)**.
The target remains 1.0.0 Stable, now subject to the four-phase hardening/release
plan rather than automatic publication after Phase 2.

| Edition | Status and purpose |
| --- | --- |
| [RC.3](1.0.0-rc.3/README.md) | New standard/SDK ownership, external compiler integration, four phases, permitted agent acts and secure human qualification; unsigned successor draft |
| [RC.2](1.0.0-rc.2/README.md) | Preserved compiler-focused candidate and original wire fixtures; its source-set capture and signed OW-WAR-0074 scope are not rewritten |
| RC.1 | Historical source whose retained acceptance record literally says `1.0.0`; original bytes/signature keep that identity |

This index is navigation, not an acceptance or a runtime source-selection change.
The original `1.0.0` acceptance bound
`sha256:b7105f5283052579f1ddfe1ab4e3ad5b1021515aeefd9fcfb21596d9a60d74e6`.
No historical signature is reused for different bytes. Each successor adoption
needs its exact source set and actual authority record.

**SAS 1.1.0, stated accurately.** `docs/sas/revisions/1.1.0.toml` records
1.1.0 as `state = "accepted"` at
`sha256:321413359168c176faaddefc1a7057d66059d5de8d98d20feeca995c7f89a1bc`,
with a signed attestation (`revisions/attestations/sas-accept-1.1.0-1.dsse.json`).
The RC.2 drafts later said the owner had withdrawn the 1.1.0 *release line*
(the batch-signing and dashboard plan). No withdrawal act is recorded, and
1.1.1, 1.2.0 and 1.3.0 were each accepted in turn with 1.1.0 in their
predecessor chain. So 1.1.0 is an accepted revision whose plan was set aside,
not a revision whose acceptance was undone.

**Proposed relabelling (OW-ADR-0032, not yet accepted).** OpenWarrant Format
1.0 (`docs/format/FORMAT-1.0.md`) becomes the normative standard. SAS
1.0.0–1.3.0 and RC.1–RC.3 become rationale and history. Nothing here moves a
signed digest: the files and revision records keep their bytes, and the
relabelling is recorded in `docs/format/README.md`, not in those files.
