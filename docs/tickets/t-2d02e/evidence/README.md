# Contractor plant copy repair

The failing `git clone --local --no-checkout` command returned 128:
`fatal: failed to create link ... Invalid cross-device link`.
Source was on `/mnt/4tb`; destination was a task-owned temporary directory on
`/tmp`. Adding `--no-hardlinks` made the same source clone successfully.

The retained regression test extracts the actual clone and frozen-path blocks
from `56-contractor.sh`. It creates a tiny real Git repository, runs that block,
checks all fourteen file bytes, and rejects shared Git-object inodes even on
same-filesystem CI. The red control found a hard-linked object; green preserved
14 files and copied 21 objects without hard links. Controls run in the plant.

A disposable worktree at `e1d4b406` used the existing copied Rust 1.97.1 CLI from
`91f912aa` to run the full contractor plant. It passed all 26 contractor checks,
including the invoice-field refusal, but its initial corpus check failed on a
stale generated projection, leaving overall exit 1. Raw output retained.
The source integration checkout was then regenerated through the compiler;
`war check --generated` reported 2,001 passes, 363 warnings, zero errors.
An exact-source whole gate remains required; no qualification is claimed here.

Local license tooling: `cargo-deny 0.20.2`, installed with Cargo 1.97.1 and
`--locked` under `/mnt/2tb/ow-gate-tools`. Its license check on this integration
checkout exits 0 and prints `licenses ok`. No license configuration was changed.
