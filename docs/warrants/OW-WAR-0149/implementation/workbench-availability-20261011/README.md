# Source workspace availability, 2026-10-11

## Established implementation bounds

The actual pre-change CLI returned process/report status 1 and one ERROR for a source worktree configuration that its actor could not read. A nonexistent source path already returned UNKNOWN. The defect was repository-opening I/O propagation, not the general CLI UNKNOWN renderer.

The fix keeps source-opening I/O failures as workspace.unavailable. Malformed TOML and invalid UTF-8 remain known refusals. The shipped-CLI controls exercise Dispatch and bundle creation: missing sources and permission-denied configuration return matching status 2 and UNKNOWN without ERROR, write no packet/bundle, and append no additional compiled-Dispatch journal event. Wrong contracts, dirty or unrelated workspaces, stale checkpoints and available malformed inputs retain their refusals. The valid checkpoint package remains readable offline.

Hosted run 38134610445 at e5c59a613c156f623f18d6425ac3dca37da6bd2d passed these controls and all 231 web tests (142.762 seconds for unittest). Rust 1.97.1 build, formatting, whitespace and strict CLI Clippy over all targets pass. Local controls timed out at their first positive Dispatch before reaching the new cases; the original 20-second control deadline is unchanged.

Both t-d1037 items are complete at claimed level only. The earlier default actor label claude remains in the journal; the tool-reported stale checkpoint claim was reclaimed by the actual actor codex. No performer field or historical event was rewritten.

## Limits

These are bounded public-command observations and fixture tests. They do not establish every filesystem failure, native provider qualification, OS isolation, an independent Warrant verdict, human acceptance or stable release readiness. OW149 remains open. The required full gate for the final integration remains pending.

Raw logs are gzip-compressed here; logs.json records their uncompressed SHA-256 hashes. The local timeout remains preserved alongside the passing hosted observations.

The final full compiler returned status 0, zero errors and unknowns, nine written projections and no skipped Warrant. The actual status result exactly matches committed CORPUS_STATUS.json after removing only the pre-existing dynamic compliance field. This checks projection integrity only.
