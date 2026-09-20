---
schema: oh.war/atom/v1
warrant_uuid: 01a0b4be-13af-7540-90a8-28d5c6d7ac59
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

1. Draft the versioned archive format decision: canonical serialization, digest
   framing, source identities, category coverage, embedded exact bytes and
   content-addressed external evidence. Preserve legacy envelope and digest.
2. Implement deterministic archive encode/decode primitives in a separate module,
   with strict version/path/duplicate/digest/size validation. Unknown optional
   extensions survive; missing required content never becomes empty success.
3. Assemble from actual current and retained records, including superseded,
   disputed and annulled states. Record provenance and unavailable references.
   Replace obsolete hard-coded absence claims with observed coverage.
4. Import into a new isolated destination without source access or authority
   activation. Resolve required evidence by verified content address, reconstruct
   IR and re-export. No caller boolean substitutes for observed reconnection.
5. Expose CLI archive/import operations; repair old false verification output.
   Retain a genuine positive round trip and named refusals in the existing control
   family. Preserve legacy format interpretation; explicit version controls apply.
6. Exercise shared KF import into an empty compatible instance and re-export.
   Record both repos/toolchains, exact source/artifact digests and runtime proof.
7. Retain implementation notes, category-by-category obligation coverage, gates,
   historical omissions and remaining formal decisions. Never self-verify.

Autonomy: prompt-authorized unverified implementation, tier T1. No signatures,
paid calls, deletion of history or silent change to pinned wire meanings. Stop
promotion of a new standard format until required format decision is accepted.
Rollback: revert new unverified source; preserve archives/evidence already made.
