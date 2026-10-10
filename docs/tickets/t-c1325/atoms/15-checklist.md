# Checklist

- [x] Reproduce public compile JSON failure with typed documents and preserve terminal progress output (i-a7e1) — done by codex, 2026-10-08: Typed-document compile now keeps progress text in human mode. Frozen executable reproduced invalid JSON; corrected build passed all 7 envelope tests, including typed-document generation and terminal progress. CLI Clippy with warnings denied passed on Rust 1.97.1. Full merge gate remains pending.
