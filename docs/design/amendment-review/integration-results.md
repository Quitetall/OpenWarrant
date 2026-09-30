# Integrated candidate validation

Observed 2026-09-14, Rust 1.97.1, in `codex/sdk-standard-amendment`.
HEAD remains `a5e9473113e9a23e00ade292e6071e26bea88dc4`; this result covers
the dirty candidate working tree, not that historical commit. The parser files
match the independently reviewed worktree byte-for-byte; their exact hashes are
in [parser-source-hashes.json](parser-source-hashes.json).

`cargo xtask gate`: exit 1, 12 of 14 steps pass. All 740 Rust tests pass, none
fail or are ignored. Build, formatting, default clippy, schemas, licenses, skill
and SPDX/workflow checks, and the 36 retained attestations pass.
The [complete command log](integrated-gate.log) retains both failing steps:

- Corpus: the README OW-WAR-0062/D-003 and xtask OW-WAR-0060/D-002 signed
  correction chains need the prepared human acts. No other corpus error appears.
- Mutation battery: the clean-input guard refuses dirty governing documents
  before its destructive restoration logic. No battery pass is claimed.

The integrated [SDK file probe](integrated-sdk-probe.log) also passed all 22 cases
with exit 0.

Independent parser review reproduced the dotted-key depth bypass and wrong NUL
span, then confirmed both fixes. It reran 13 public tests and 22 actual-file
cases, plus the original crash at depth bounds 64/128, quoted keys, mixed arrays
and tables, array-table chains, and unsupported limits 129/usize::MAX.
Its bounded verdict was PASS; this is review evidence, not a recorded Warrant
disposition, secure human acceptance, exhaustive fuzzing or assurance mark.

The SAS draft check, six footer witness tests, eight archive regression cases,
archive Git-ref/baseline restoration check, and structural eight-skill audit pass.
Fresh harness activation and model evaluation of the skills remain not run.

No human correction, SAS acceptance, new commit or main-branch publication was
performed in this implementation pass. Author/edit operations remain OW-WAR-0076.
