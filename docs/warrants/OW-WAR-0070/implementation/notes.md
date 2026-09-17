# OW70: human-waiting inbox

Source: `503dc044c9ec2c9308dd13569372bc3d109a6ebc`.
Implementation completed, unverified. No authorization, ADR adoption, independent
assurance disposition, qualification or legacy resolution is recorded here.

`war inbox` lists one row per Warrant waiting on a human signing request or open
blocking hotline question. It preserves existing phase and readiness semantics,
shows last recorded transition age (or unknown), and sorts known waits oldest
first with alias ties. It reads records without modifying them. Malformed record
containers, loader failure collections, atom errors and mismatched question or
journal ownership refuse instead of hiding required acts.

[Command reference](../../../cli/inbox.md) describes exact classification,
timestamps and provisional JSON. OW-ADR-0018 remains proposed; candidate schema
lives under `conformance/fixtures/inbox`, outside the frozen `war schemas` pack.
Human acceptance of that ADR remains a separate adoption step. The generic paths
in the historical work order map to `crates/openwarrant-cli/src/inbox/classify.rs`,
`src/inbox/mod.rs` and existing `src/main.rs` registration; no extra CLI framework
or second parser was introduced.

## Evidence and limits

- 16 focused classifier/CLI tests pass on Rust 1.97.1. Synthetic records cover
  human, agent and none; the pure classifier covers all seven phases including
  gate. The legacy loader cannot persist every core phase; no new event or
  authoritative state was invented to make a fixture look complete.
- Candidate schema validates real CLI output and its JSON roundtrip. Twelve
  malformed controls refuse. The script requires date-time validation support
  and refuses to silently skip it.
- Full local gate passed 14 steps and 308 controls **before** the final two
  repository-container guards. The final guards then passed focused tests and
  Clippy. The protected-main PR must pass its full gate on the exact final head
  before merge. The attached local log is named to preserve this boundary.
- Separate-context Spec review passed; Standards review found and confirmed
  fixes for tolerant-loader gaps, then passed. These are code reviews, not
  formal assurance dispositions.
- Actual local LAMU `review_commit` for the source returned PASS WITH NITS.
  Its age suggestion already appears in the reference: journal order, not time
  since human eligibility. The other two items explicitly confirm intended
  sorting and answer precedence. No code defect remained after checking them.

No production Warrant contract or signed record changed during implementation.
This new work report records implementation completion only. Existing history
and qualification remain unchanged. Stable 1.0 is not published by this work.
