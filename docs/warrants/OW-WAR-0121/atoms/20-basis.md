---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eca-7b20-86b5-8a94924160af
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS §86: file-native commands SHALL use temporary files, fsync where
  required by policy, atomic rename, prestate digest checks, and no partial
  generated parent publication. Cross-Holder changes SHALL eventually use
  Liminal RepairPlan and ILRP, not an OpenWarrant mutation coordinator.
- SAS §87.2: parsers and controlled writes SHALL avoid path traversal and
  symlink races.
- SAS §66.1–§66.2: the journal is append-only local draft history.
- `docs/design/openwarrant-product-spec.md`, "Engineering contracts still
  to specify": local storage, crash recovery, retained artifact migration.
- OW-WAR-0114's gap table: "Storage, crash recovery, retained-artifact
  migration", placed at `roadmap://OW-PHASE-1/storage`.
- The existing hand-written atomic writers named in 10-intent, which the
  helper replaces only where this Warrant declares the file.
- OW-ADR-0021: this Warrant declares eight existing source files. On
  authorization it governs them for the changes it makes.
  `journal_cmd.rs` is pinned by resolved OW-WAR-0031 and `verify.rs` by
  resolved OW-WAR-0046; this Warrant declares them because it must change
  them.

## Assumptions

- A-001: rename within one directory is atomic on the filesystems the tool
  supports (Linux ext4, btrfs, xfs; macOS APFS). Confidence: high for
  Linux; medium for macOS, which the plants do not run on.
- A-002: a debug-build-only fault hook (`OPENWARRANT_FAULT`, compiled out
  of release builds by `cfg(debug_assertions)`) is an acceptable way to
  stop the process at a named point so a plant can observe a crash
  deterministically. The gate runs the debug binary
  (`docs/gates/software.repo.war-check@1.0.0.yaml` argv). Confidence:
  medium; the owner may prefer the hook behind a cargo feature.
- A-003: fsync on every record write costs milliseconds per act, within
  the 60-second routine target (OW-WAR-0118 measures it). Confidence:
  high.

## Unknowns

- U-001 (non-blocking): whether `war` should offer a repair for a torn
  journal tail. This Warrant names the truncation and refuses to do it. A
  repair command would edit a journal, which `journal.rewritten` refuses
  for committed lines; a torn tail is by definition uncommitted.
- U-002 (non-blocking): which writers "policy" requires fsync for (§86).
  This Warrant fsyncs every writer it routes. A policy knob is later work.

## Residual risks

- R-001: a crash between two files of one act (a record written, its
  journal line not). Each file is whole; the pair can disagree. Named in
  Non-goals, and in `docs/STORAGE.md` with the findings that detect each
  known pair.
- R-002: `check.rs` is also declared by OW-WAR-0114 (D-013) and OW-WAR-0119
  (D-002). The later authorization governs it. The performer rebases on
  whichever lands first, and each Warrant's plants are the check that the
  others' rules survive.
- R-003: the fault hook is code in the product. It exists only in debug
  builds; a plant asserts a release build ignores `OPENWARRANT_FAULT` if
  one is present, and `UNKNOWN` is reported when none is.
