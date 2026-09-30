# Read-only board tree observations

The one-shot `war board` now enables the existing process-local tree-read cache already used by `war status` and `war next`. Interactive console and execution paths remain outside this cache. The cache stores successful Git reads only; it does not grant authority, refresh the caller index, or reuse results across processes.

The existing board made 1,216 Git scans: 608 untracked scans and 608 diffs across five prior trees. The actual command control fails against that binary, then observes exactly six distinct scans once each with the patch. Caller index bytes remain unchanged.

On the unchanged main-consolidation checkout, the original command took 18.931 seconds and the patched command took 5.183 seconds. JSON bytes are identical (SHA-256 b7d22dda3e8c406d29b89f0ae34ca22f79d25d6058e09d4b56968b4ca6b21a37). These are single local observations, not a universal latency guarantee.

Rust 1.97.1: the public CLI parity/refusal test passed. The two targeted public HTTP parity/authentication tests passed with the original ten-second subprocess bound. PR #136 had four timeout failures before the patch. Full final-source CI is still required. The ticket is completed unverified; no Warrant disposition or signature was written.
