# Current preservation observations

W84 remains implementation complete and unverified. These observations recheck
the public preservation and successor boundary without changing original records.

Eight public library tests and ten file-backed codec cases pass. The new observer
retains 24 CLI calls. All 25 files in the resolved historical fixture match exact
Git bytes from `faaf3a4aa70029600ecfed58aab53177cefa8c10`. The observer feeds those
Git bytes to the SDK and checks the returned inventory. It observes refusal of a
changed signature byte and a missing signature file.

Fresh documents preserve Unicode, CRLF, binary data and a copy of the original
signature at its explicit archive path. A successor keeps both code versions and
reports `authority_transferred: false`. Self-successor, duplicate, unchanged,
missing and traversal paths refuse. No signed source file is rewritten.

The [source observation](current-preservation/source-observation.json) identifies
the exact inputs, toolchain and reused CLI binary. This branch changes observation
files only; the SDK and CLI implementation matches the sealed W83 consumer build.
The adjacent wire transcript retains actual requests, responses and exit codes.
[Fixture instructions](../../../../conformance/sdk/legacy/README.md) describe
reproduction.

This proves preservation of the named inventory, not the full external historical
artifact graph. Source digests do not authenticate signatures. Archived signature
bytes grant no new authority. No independent verdict, human acceptance, assurance
mark or full current gate pass is claimed. Original evidence and history remain.

Next: W85's full current-host SDK and skill inventory, then the remaining provider
and release qualification. Warrant resolution remains a separate human act.
