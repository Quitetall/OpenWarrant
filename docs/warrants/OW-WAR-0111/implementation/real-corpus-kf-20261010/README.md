# Real OW11 corpus through Knowledge Fabric — 2026-10-10

The complete real OW11 archive passed the existing external-archive KF preservation drill at merged KF revision `75390376b42a0e4d60bd74d67184e88df3e09eaf`. No KF source changed. The input adapter's existing filename and test label are `native-runtime`; the supplied bytes are the recovered real OW11 corpus, not a new native provider execution. Source-derived identity and runtime sidecars came from the checked archive and OW consumer.

The selected test passed in 130.035 seconds (Vitest total 135.15 seconds). The two original baseline scenarios were unselected, not claimed rerun. Node 24.21.0 satisfies the repository's `>=24.18.1 <25` engine range; pnpm 11.21.0, Vitest 4.1.11. Offline frozen-lockfile dependency installation and `tsc --build` passed. Initial missing executable-bin warnings preceded the successful build; no source was changed to hide them.

The drill uses two fresh PostgreSQL instances and isolated object storage. It populates all six Warrant record families and superseded/disputed/annulled synthetic fixture records, stops the source database and storage, restores an empty compatible instance, checks signature/trust and missing-section refusals, and recovers exact contract revisions plus the version-pinned original source archive. Fixture signing keys and actors confer no authority over actual OW11.

Recovered archive: 24,755,820 bytes; SHA-256 `c6c38432b92dcb5980783ac3d2817919e3652a4c7cc5fc0cac5e37f37d525967`. Its bytes match the original full corpus archive exactly. From an empty directory, the OW consumer imported the KF-restored bytes, re-exported the identical archive and reconstructed the runtime basis. Raw logs, source-derived sidecars, signed synthetic export package and restored archive remain under `/home/brianklam/Projects/OpenWarrant/docs/runtime-evidence/native-kf-recovery-20261010/`.

This completes the real corpus producer/consumer preservation observation, not independent qualification, human acceptance, archive-format adoption or Warrant resolution. OW111 remains in progress pending the combined main gate and formal acts.
