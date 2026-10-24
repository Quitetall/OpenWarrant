# W30 legacy export JSON response

**CLI transport corrected; archive scope remains incomplete.** Ordinary failed
legacy export printed a human prelude followed by a JSON error; `--force`
printed only human text. Neither was one JSON response for an API caller.

The wrapper now uses the existing report renderer in JSON mode. The result is
one `oh.war/report/v1` error envelope stating that the legacy metadata is
incomplete and no archive was written. Ordinary failure still exits 1; forced
failure still exits 2. [Byte comparisons](human-compatibility.json) prove that
both human stdout and stderr remain unchanged. No exporter, canonical preimage,
record schema, content digest, import behavior or archive format changed.

Both public regression cases fail before the correction and pass afterwards.
The complete JSON response suite passes 13/0; all-target Clippy with warnings
denied and formatting pass. The first full-suite run found generated progress
stale after the source edit (12 pass, 1 fail). Public compilation corrected the
projections; that failure and the final passing run are retained. The initial
formatting mismatch was also corrected before the final checks.

This is ordinary unverified wrapper work in lib.rs, not a declaration that a
new W30 archive deliverable has been accepted. All three original preservation
obligations remain unestablished: complete content export, real import/re-export
and full historical preservation. The [preservation gap](../preservation-gap.md)
still defines the required route. The current IR codec retains structured
sections, but source paths/digests are not source bodies; the legacy portable
metadata envelope has no content-bearing collection. W111's experimental
archive is a separate transport, not silent adoption of a replacement for the
frozen envelope and war_export_digest.

W30 remains open. No signature, authorization, independent disposition or
delivery pin was changed. Scratch from this run was removed; durable source,
binary and observations remain under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow30-export-json-20261024/`.
