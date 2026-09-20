# Preservation implementation gap — 2026-09-18

Inspected main 1eb2cef4 and matching export source in ce6ef1f (OW110 gate build).
Command: `war export OW-WAR-0030 --round-trip --reconnect`.
Observed exit 0 and:

```
OW-WAR-0030: §68.3 round trip verified (sha256:76e24642916ab1c980ce89102eadbf6e0dfd625958954dd308feecaf549988f3)
```

This is a false preservation claim. `export::round_trip` invokes `assemble` twice
on the same repository. No archive is serialized or imported; the caller's boolean
becomes `evidence_reconnected`. `assemble` hashes concatenated current atom bytes,
counts records, and marks categories present without embedding their contents.
Four categories are always marked absent using obsolete repository-wide claims.
`--force` says a package *would* be written; it does not emit an archive.

The positive battery control in `00-corpus.sh` expects this false success. Do not
cite it as proof of import, historical preservation, or evidence reconnection.
Tests that construct matching digest strings also do not establish those acts.
OW30 remains incomplete; no original assurance or signed history is changed here.

## Required implementation

1. Preserve the existing export envelope and digest meaning as historical v1;
   do not silently repurpose fields or alter old recorded bytes. Prepare an
   explicit format decision for a versioned content-bearing archive and import
   contract. This is required by OW30's frozen surfaces and CONTRIBUTING's
   canonicalization rule.
2. Inventory actual records per Warrant, including retained contract revisions,
   atoms, manifest, IR/basis, applicable decisions, action/audit and runtime
   evidence, artifacts, assurance, resolution/standing, and signature/checkpoint
   records. Distinguish absent, external-but-addressed, and embedded records.
   No hard-coded claim that nothing executed or resolved.
3. Define canonical digest framing and source-relative paths. Refuse traversal,
   symlink/special-file input, duplicate paths, digest mismatch, size exhaustion,
   unresolved required evidence, unsupported versions and incomplete categories.
4. Import into a new isolated destination, reconstruct IR from retained inputs,
   and re-export from that destination after removing access to the source.
   Compare canonical bytes and semantics. Never trust a reconnect boolean.
5. Extend the existing round-trip test/control with a genuine complete fixture;
   its successful import/re-export must remain possible. Add missing populated
   section and tampered evidence controls. Preserve superseded, disputed and
   annulled historical records, not only current state.
6. Report implementation evidence separately from authority/verification. Imported
   signatures remain historical claims until validated against retained trust
   material; importing records grants no local execution authority.

Current pin inspection: `crates/openwarrant-core/src/journal.rs` is resolved
OW-WAR-0031 D-001 (one correction recorded). The CLI `export.rs` was not listed as
resolved-pinned. Recheck pins before implementation. Use a separate versioned
module if that preserves the frozen contract; editing pinned source still needs
its proper correction record. This audit grants no signature or new wire meaning.
