# Program-specific phase projection

Completed bounded implementation and publication work, unverified, 2026-09-30.
Implementation revision: `31c8a28338ab387cafd12201ee0f70892874dffa`.
Candidate: <https://github.com/Quitetall/OpenWarrant/pull/135> (draft).

The section-98 parser now accepts combined and separate objective/exit table
columns, while retaining existing heading/exit-bullet input. Corpus status uses
the configured program's own phases regardless of count. Missing or conflicting
declarations do not import the framework catalog. Undeclared roadmap references
remain visible with unknown exit criteria and a caveat. No assurance or phase
acceptance is inferred from these declarations.

Observed red controls: table input previously returned no phases; CLI fixtures
with two or zero declarations previously returned framework objectives. After
the fix the parser controls, existing heading tests and four CLI controls pass.
A separate program's 11 authored titles and exits exactly match regenerated
JSON, without changing its pinned SAS bytes. The caller's private source and
receipts are not included in this public change.

## Checks

Pinned Rust 1.97.1, two compiler jobs, debugger symbols disabled, isolated target.
`cargo xtask gate` completed with 12 of 14 steps passing: 924 workspace tests,
formatting, Clippy, schemas, SPDX/workflows, licenses (cargo-deny 0.20.2), and
111 DSSE attestation checks passed. It did not pass as a whole: corpus checks
report four moved resolved source pins, and the initial plant step refused dirty
authored records before the candidate commit.

After committing the candidate, the default plant battery completed with 314
passes and six positive-control failures against the still-drifting corpus.
It is not a passing battery. The optional slow neighbour-timeout plant was not
enabled. A preceding attempt was terminated after diagnosing its nested Cargo
build using the global target; its log remains retained. The corrected attempt
used:

```sh
env RUSTC_WRAPPER= \
  CARGO_TARGET_DIR=/mnt/2tb/cargo-target-openwarrant-program-phases \
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  bash conformance/plant.sh
```

Local retained logs live under `/mnt/4tb/lamu-platform-proof/`:
`openwarrant-program-phases-gate-20260930.log`,
`openwarrant-program-phases-gate-20260930-run2.log`,
`openwarrant-program-phases-plants-20260930.log`, and
`openwarrant-program-phases-plants-20260930-run2.log`.
These are developer observations, not independent qualification.

## Human correction boundary

The unchanged resolved records pin these source artifacts:

| Warrant | Deliverable | Request |
|---|---|---|
| OW-WAR-0055 | D-003 | [request](correction-0055-D-003.json) |
| OW-WAR-0057 | D-002 | [request](correction-0057-D-002.json) |
| OW-WAR-0058 | D-001 | [request](correction-0058-D-001.json) |
| OW-WAR-0062 | D-005 | [request](correction-0062-D-005.json) |

Current `status.rs` SHA-256:
`bfe815d4078937cb7bfb6ad03133ea053e0e5176b70d39cff0aba12d7b00f982`.
Current `sas.rs` SHA-256:
`4154dc4cf6e90f27ce2686d4c8e49aa1486db68ad39d9686feda6d40cc93eeaf`.
Requests are emitted by `war correct ALIAS DELIVERABLE --json`; they are not
signed responses. No resolved pin, disposition, authorization, attestation,
signature or resolution was edited. Human corrections, independent review and
the complete gate remain required before merge. The installed CLI was not replaced.
