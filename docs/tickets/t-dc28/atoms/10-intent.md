# Collision-free ids for amendments and plants

Tickets use hash ids (t-xxxx). Amendments (AM-NNN) and plant files (NN-*.sh) are still sequential and collided this week (0137's AM-002 renumbered to AM-003; two 57- and two 58- plants).

## Notes

- **2026-09-25 23:03 UTC, claude:** Side fix: main.rs every-subcommand test now builds the clap tree on an 8 MiB thread; adding war amend pushed the unoptimized tree past libtest's 2 MiB stack (passes at 2.2 MB). The next top-level subcommand would have hit it anyway. Full battery in a clone: 1123 passed, 5 failed — all 5 in 89-schemas/90-skills, exit 101 from cargo trying an sccache RUSTC_WRAPPER the battery env inherited; re-run with RUSTC_WRAPPER= : 12/12.
