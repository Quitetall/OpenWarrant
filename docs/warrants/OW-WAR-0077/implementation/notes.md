# OW-WAR-0077 implementation

Base: 8f688e1e531a0a9bc7a5711a78c4583d8cc09f3e. Work in progress, unverified.

Public seam: RC.3 format F3/F5 source descriptors and bound references, checked
against explicitly supplied bytes. Source acquisition, path confinement at I/O,
source-change detection during capture, dependency selection and authority checks
remain provider/caller responsibilities. The SDK must not hide these operations.

Phase 1: typed descriptors/identities/references, bounded codecs, source integrity
and exact declared range checks, direct file-backed conformance driver.
Phase 2: explicit provider response integration; no LAMU support claim without
its actual implementation passing interoperability cases. Initial local LAMU
source inspection found no matching OpenWarrant capture/resolution endpoint.

Requested future workflow: a checklist and start-work actions in the web viewer.
This remains outside 0077 and the read-only progress viewer; not implemented here.
