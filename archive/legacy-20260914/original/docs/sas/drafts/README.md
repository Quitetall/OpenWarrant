# OpenWarrant SAS editions

Current candidate: **1.0.0-rc.2**. Target after Phase 2 conformance: **1.0.0 Stable**.
This is refinement of 1.0, not a new 1.1 release.

| Edition | Source and status |
| --- | --- |
| [1.0.0-rc.2](1.0.0-rc.2/README.md) | Consolidated candidate: standard, compiler, assurance meaning, later workflows, examples, Phase 1 build scope. Not accepted or implemented merely because written. |
| [1.0.0-rc.1 baseline](../WAR_Software_Architecture_Specification.md) | Earlier source retained byte for byte; owner corrected its release designation. |

The existing [acceptance record](../revisions/1.0.0.toml) literally used `1.0.0`
and bound `sha256:b7105f5283052579f1ddfe1ab4e3ad5b1021515aeefd9fcfb21596d9a60d74e6`.
The original source, record, and signatures retain those bytes. The rc.1 designation
does not claim the original signature named a release candidate.

The configured repository source and installed tool still use that historical
record. RC.2 adoption and any registry/version migration require exact source-set
acceptance through the existing authority process. No old signature is reused as
acceptance of different source bytes. The old unaccepted `drafts/1.0.0/` working
copy has been consolidated into the explicitly named RC.2 directory.

Stable requires feature/library and CLI/compiler proof. Workflow integrations,
starting with Knowledge Fabric Compiler, follow that release boundary.
