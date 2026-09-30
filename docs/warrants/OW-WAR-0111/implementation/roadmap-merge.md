# Roadmap main integration

Integrated main `7e9b9752bdb0febfffe3c2b1a289b4f718de874a` into the OW111
implementation branch after `a4dccb09`. Only generated CORPUS_STATUS HTML/JSON
conflicted. Both authored inputs were retained and the projections regenerated
with the combined-source `war compile`; no generated hunk was hand-edited.

Initial shared-target build used a stale compiler artifact without preservation.
The source module declaration was present. Updating that source file's mtime
forced a compiler rebuild without changing bytes; the combined build passed.
On Rust 1.97.1, all 15 preservation and eight viewer integration tests passed.
Generated checking reported 966 pass, 88 warnings, zero unknown and zero errors.
These are bounded merge checks, not independent assurance or a full merged gate.
