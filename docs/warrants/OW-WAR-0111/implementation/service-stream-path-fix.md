# Service receipt stream paths

A real `war run OW-WAR-0066 STAGE-001` in a disposable clone returned three
passing diagnostics, but both receipt stream references were missing. The gate
runner wrote streams in the Warrant directory; receipt minting named `gate-runs/`.
Archive auditing exposed the inconsistency. Original broken archive and run report
are retained; no historical receipt was rewritten.

The runner now receives the same `gate-runs/` directory used for run records and
receipt minting. A fresh-repository regression executes a service emitting distinct
stdout and stderr, reads their exact bytes through receipt references, and checks
that archive export retains those same bytes. It also refuses the old root-level
stream placement by asserting those stray paths do not exist.

Validation: 17 preservation tests passed; all-target CLI Clippy passed on Rust
1.97.1. Generated checks: 966 pass, 88 warnings, zero unknown/errors. Before this
fix, full gate at 9062bd41 passed 14 steps and 309 planted checks; that older full
pass does not establish this fix's full integration gate.

The disposable service gate uses `true`; it proves the execution seam, not feature
correctness or actual project completion. Runtime category collection, journal
`dispatch.compiled`/`submission.recorded` reference resolution and historical
stage-to-contract bindings still require work. No authority, signature or
independent assurance was granted.
