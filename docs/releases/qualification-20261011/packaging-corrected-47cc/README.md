# Corrected native packaging rehearsal

Hosted run [38149192266](https://github.com/Quitetall/OpenWarrant/actions/runs/38149192266)
completed successfully on source `47cc456348ae0ebe4291177647b6f563fb191c21`.
Its gate, Linux x86-64 build, macOS arm64 build, installed walkthroughs and
cross-host artifact checks passed. The 145-entry IR lists are byte-identical;
the packaged SDK source payload is also identical. OS-specific binaries and
license inventories remain distinct.

Both actual hosted install receipts contain eight successful commands: installed
skill links, version, SDK validation, interactive authoring, prototype init,
Warrant creation, compilation and HTML overview. Re-downloading and replaying the
Linux package here passed the same eight steps. The wrong-checksum control refused
with `ValueError: Archive checksum mismatch` (exit 1), without writing a receipt.
Temporary installations were removed; host PATH, authority and services were not
changed. No model was called.

[Observation and file identities](observation.json) preserve the exact scope.
The original downloaded archives and artifact directory structure remain in the
project evidence store named there; the repository retains compressed original
candidate/install receipts, job metadata and complete hosted log. These are
performer observations. The local re-download is not independent verification.

The packages explicitly report `qualified: false`, `promotable: false` and
`binary_source_linkage: caller-supplied-unestablished`. Successful CI and matching
checksums do not replace independent release build provenance. Publication,
crates publication and alpha publication were skipped. This rehearsal creates no
tag and grants no assurance or release permission. Later code is not qualified
by this older subject.

The earlier missing-notice failure and subsequent stale-projection failure remain
in their original evidence files. This successful corrected subject supplements
them; it does not change their results. Final phase review, actual users and
OW90/OW91 qualification and permission remain pending.
