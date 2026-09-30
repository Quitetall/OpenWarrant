---
schema: oh.war/atom/v1
adr_uuid: 9b2f6a41-7d3e-4c58-a1f0-5e8c2d7b3a19
local_alias: OW-ADR-0014
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a021a2-b570-7f57-85b2-0f8189873d9e"
---

# ADR OW-0014: an async runtime is admitted for the MCP transport, and nowhere else

## Status

Proposed by the performer under the 1.0 plan (slice A1); adopted when the owner
accepts it.

## Context

The workspace `Cargo.toml` records, beside `ureq`, why `war` has no async
runtime: it is a CLI, every act is a blocking call over files, and pulling
`tokio` in to make one HTTP request was a larger change than the §67 seam
justified. That reasoning still holds for every command that exists today.

1.0 adds `war mcp`: a Model Context Protocol server over stdio, so an agent
harness that has never heard of OpenWarrant can call `war_check`, `war_next`
or `war_plan_request` as tools. The reference Rust SDK, `rmcp`, is async
throughout — its server trait, its stdio transport and its macros all assume a
runtime. Reimplementing JSON-RPC framing, the initialize handshake and the tool
and resource schemas by hand would trade a runtime dependency for a protocol
reimplementation this program would then have to keep conformant on its own.

## Decision

- `rmcp = "=3.3.0"` with `default-features = false` and only `server`,
  `macros`, `transport-io`: no `auth`, no HTTP client, no child-process
  transport. `tokio` with only `rt`, `io-std`, `macros`: a current-thread
  runtime built inside `war mcp` and dropped when it exits. `schemars`,
  which `rmcp`'s server needs for tool input schemas.
- Async is confined to `crates/openwarrant-cli/src/mcp/`. Every tool body is a
  synchronous call into the module that already implements the command; the
  runtime exists to drive the transport, not the work. A unit test greps every
  other source file of the workspace for `tokio`, `rmcp` and `async fn` and
  fails on the first hit, so the boundary cannot erode without editing the test.
- The comment in `Cargo.toml` is amended from "no async anywhere" to "no
  async outside `openwarrant-cli/src/mcp/`", citing this ADR.

## Evidence

Measured on the 1.97.1 toolchain against commit 4e8c901, with the lockfile
updated minimally (no unrelated crate moved):

| | before | after |
|---|---|---|
| crates in the normal dependency graph (`cargo tree -e normal`, unique) | 86 | 108 |
| licence families (`cargo deny list`) | 10 | 10, the same ten |
| `cargo deny check licenses` | ok | ok |

The families are AGPL-3.0-or-later (this workspace), Apache-2.0, BSD-3-Clause,
BSL-1.0, CDLA-Permissive-2.0 (the CA bundle already excepted in `deny.toml`),
ISC, LGPL-2.1-or-later, MIT, Unicode-3.0, Unlicense. No new family, so no new
exception. A future `rmcp` bump that introduces one is a refusal at
`cargo deny`, not a case for an exception.

## Consequences

- `war mcp` can be built on the reference SDK and stay conformant as the
  protocol moves, at the cost of twenty-two more crates in the release binary's
  graph. The pin is exact (`=3.3.0`); a bump is a deliberate commit with the
  table above re-measured.
- Every other command stays exactly as it was: blocking, runtime-free,
  testable with a scratch directory. Nothing outside `mcp/` may spawn a
  runtime or await.
- The refusal of async recorded beside `ureq` is narrowed, not reversed:
  §67's HTTP seam still makes its one request with `ureq`.
