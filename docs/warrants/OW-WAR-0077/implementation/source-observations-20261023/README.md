# W77 current source observations — 2026-10-23

Performer observations; independent verification and human assurance remain
separate. These records do not replace original implementation history.

- Eleven Core source tests and all 19 supplied-source fixture cases passed.
  The native driver calls public SDK functions; its binary/source identity and
  complete output are retained here.
- The actual optional LAMU source provider passed all five filesystem integration
  tests at committed participant `2572894f28ce76c2c7e6c6c9ae6be51a7f400fa1`.
  `provider-observation.json` records the complete commands, output, clean source
  status, source hashes, SDK pin, shared-contract digest and limitations. No LAMU
  daemon, database or model was started. This run establishes Linux behavior only.
- The CLI source-lock regression first failed because the operation was absent,
  then passed against the unchanged canonical F5 vector, reversed enumeration
  and changed-byte refusal. Original failure and corrected run are retained.
- `wire.jsonl` contains all 65 actual public SDK request/output/exit observations
  over 19 unchanged source fixtures, extra unseen binary bytes and refusal to
  overwrite an existing output. `summary.json` binds the wire and runtime;
  `controls.json` contains the extra control data. Per-case decoded copies remain
  in the durable run archive, rather than duplicating the wire in this document.

The CLI binary was built from the working candidate described in
`cli-identity.json`, not claimed a clean HEAD build. The source-provider runtime
uses its contract's exact published SDK revision; it is not claimed to link the
current working-tree SDK. Canonical locks are supplied-data integrity results;
metadata labels remain claims and no permission, context completeness or human
assurance follows these observations. The provider's sequential rechecks do not
create an atomic multi-file snapshot.

The complete CLI integration recheck passed 359 tests with four ignored. Strict
all-target/schema Clippy and formatting passed. The first integration run had
358 passes and one corpus-projection mismatch: the source edits changed stale
source-reuse path counts. The tool regenerated those projections without
refreshing old evidence or altering any expected refusal. The original failed
run remains in the durable run archive.
