# Program phase compatibility validation

Performer observations only. These are not independent verification dispositions,
authorization, SAS acceptance, or Warrant resolution. Baseline is
`8bc3978a26de0c13605c6d206fe820a8440be737`; final commit identity is assigned after
this report, then reviewed independently through LAMU.

## Results

| Observation | Result | Bound |
|---|---|---|
| Original clean `/mnt/4tb/OpenWarrant`: `war check --generated` | exit 0; 624 pass, 78 warn, 0 unknown, 0 error | Baseline sources and existing binary at base 8bc3978 |
| `cargo test -p openwarrant-core -p openwarrant-cli` | exit 0; 476 core and 114 CLI tests passed | Includes signed grammar, all fourteen LIM phases, OW 0–10, missing/ambiguous/drifted authority, duplicate/malformed declarations, fenced/quoted examples and false completion prose |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 | Rust 1.97.1 |
| `cargo xtask gate` | exit 1; 7 of 9 steps pass | Build, formatting, clippy, tests, licenses, SPDX and workflow checks pass; corpus refuses 10 digest changes; plant battery refuses dirty generated documents |
| Recompiled generated views | no generated drift | `war check --generated` still exits 2 because of the 10 digest changes, not generated drift |
| Same-source old/new `war status --json` | existing Warrant and requirement records equal; all 11 real objective achievement values equal | 22 satisfied requirements, 2 superseded, recorded phases 1 and 6 retained; new authority field and expanded synthetic unassigned text are intentional |
| Original SAS, config, ADRs and authorized Warrant records | no authored-source changes | Generated projections change through `war compile` only |

`CARGO_BUILD_JOBS=2` and this worktree's isolated `target/` were used. Full gate
output and original baseline output are retained beside this report. An earlier
exploratory plant run observed the seven new refusal cases but used an earlier
implementation and failed six overall cases; it is not final qualification.

## Adoption blocker

All 10 candidate digest errors are newly introduced by this bounded code change;
none occurs on the clean baseline. `adoption-blockers.json` records each prior
Warrant, deliverable, target, immutable prior digest, and candidate digest.
OW-WAR-0064 already describes the missing authorized correction act, including
this namespace defect. Neither that act nor a human disposition is fabricated
here. Existing deliverable, authorization, verification and resolution records
remain unchanged. An independent final review and authorized correction route
must precede adoption; a commit or regenerated projection cannot clear this gate.

Draft next-actionable entries remain structural authoring hints, explicitly
labelled as not execution authorization. No runtime dispatch was performed.
