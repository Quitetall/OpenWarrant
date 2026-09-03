---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f2a-8e39-69730f255e33
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

Obligations, not one prose claim (SAS §38.1, RQ-050). Each states its bounded
scope, because a claim is bounded by its evidence (§38.4, Law 14).

## Acceptance Obligations

### OBL-001 — the workspace builds on the pinned toolchain
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** existential over the workspace members, on Linux x86-64 only.
  No claim is made about macOS, Windows, or any other toolchain.
- **evidence:** `cargo build --workspace` exit status, with `rustc --version`
  captured in the same run.

### OBL-002 — the aggregate gate passes
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** the four steps `cargo xtask gate` declares — fmt, clippy, tests,
  licenses — at the commit under test, on 1.97.1.
- **evidence:** `cargo xtask gate` exit status and its per-step report.

### OBL-003 — the gate has been observed to REJECT
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** the gate as a control, not the code under it.
- **evidence:** a recorded run in which `cargo xtask gate` exits non-zero and
  names the failing steps.
- **why this obligation exists:** OBL-002 alone is satisfiable by a gate that
  checks nothing. A `gate()` that returned `ExitCode::SUCCESS` unconditionally
  would pass OBL-002 on every commit forever. Only an observed rejection
  distinguishes a working control from a decorative one, and this repository's
  parent project has shipped a green gate that compared nothing three times.

### OBL-004 — `war init` initializes a repository, and refuses to reinitialize
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** the `war` binary, invoked as a process. Unit tests do not satisfy
  this obligation; §38 distinguishes a performer's internal claim from an
  observation of the delivered artifact.
- **evidence:** two runs — one creating `openwarrant.toml` plus the four
  configured directories and exiting 0, one exiting non-zero against an
  already-initialized repository with the original namespace intact afterwards.

## Gate Adequacy

Adequacy review is not required at this assurance level (`basic`, §25.1). The
adversarial question was asked anyway and is recorded because the answer is
uncomfortable:

> Could every obligation above pass while the delivered system is useless?

Yes. Nothing here requires the compiler to parse a manifest, so a repository that
builds, gates, and initializes but cannot read a single Warrant satisfies all
four. That is accepted deliberately — it is the scope of this Warrant, and
OW-WAR-0002 is where it stops being true.

## Residual Risk

- CI runs on `ubuntu-latest` rather than the self-hosted runner the
  private-repository tier calls for. Accepted; the exposure is bounded by
  `timeout-minutes` and the gate is one short job.
- The `gate` job is not yet a required status check on `main`, because branch
  protection for this repository has not been applied. Until it is, the gate is
  advisory: a red run does not block a merge.
