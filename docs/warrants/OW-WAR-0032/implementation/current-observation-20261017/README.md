# Schema and KF consumer observation

Read-only observation on 2026-10-17. This report adds current evidence; it
does not change any received verdict, signed contract or resolution.

The OpenWarrant record source is `c0385cdbd4f3de38f62730fe1343dbf01ed6ded4`.
Its Rust code is identical to `9e4630f83d9993629f3d6e92fb0eb657e0011d66`.
The actual checkout CLI's `admin schemas --check` passes: 25 schema families,
their TypeScript projections and the pack match the source types. The active
pack remains `openwarrant-schema-pack` version `0.2.0`.

KF's clean checkout is `dabd1065392db8c55630000c71c984ad098f75ae`. It imports
OpenWarrant's generated submission type in `packages/warrants/src/submission-contract.ts`.
The real submission action imports that module from `projections.ts`.
The warrants package's normal TypeScript build includes these sources.
Its public package exports generated Dispatch and Submission types.
The runtime validator retains its own refusal checks; TypeScript is not
runtime validation or authority.

All 15 retained KF declaration files match their declared artifact hashes
and the actual OpenWarrant Git source named by KF's source record:
`8d8b81ce6704eff8bbcc9e5dd36cd689021f64b7`. KF pins an earlier producer
edition; this is not a claim that it consumes every current schema family.
KF's exact-source hosted CI [38217114124](https://github.com/Quitetall/openhuman-knowledge-fabric/actions/runs/38217114124)
passed, including its clean build, typecheck and test jobs. These are existing
hosted observations; no new local KF build or runtime session was run here.

The earlier gap report under OW-WAR-0090 describes an older source. The
TypeScript generator and KF import gaps it names have since been addressed.
The scope map under OW-WAR-0110 remains useful for the other obligations:

- **OBL-001 is incomplete:** the schema entry list still lacks the requested
  gate family. Successful generation alone does not prove that all existing
  corpus records validate against the specified families. The actual pin report
  identifies `crates/openwarrant-core/src/gate.rs` as OW-WAR-0019/D-001,
  resolved and not historical. Adding schema derives there requires the
  applicable correction or successor route; no pinned bytes were changed.
- **OBL-002 is incomplete:** the historical obligation names a
  `FormatBasis.digest` field. The current model uses other format pins and a
  separate pack digest. This report does not change frozen digest semantics
  or treat that difference as accepted reconciliation.
- **OBL-003:** the current drift check passes for unchanged source. The
  earlier JSON and TypeScript refusal observations remain in their original
  records; this check did not rerun those plants.
- **OBL-004 now has stronger source evidence:** generated declarations are
  imported by the real KF boundary and its exact source has passed hosted
  builds and tests. An independent verifier must decide whether the retained
  evidence satisfies the obligation.

Six TypeScript emitter tests and two schema-pack tests pass against the
compiled code candidate. They cover required/nullable/tuple shapes, local
references, unsupported-construct refusals, deterministic generation and
the version inside contract digests. Raw test output and argv are retained;
these unit checks do not substitute for corpus validation or independent review.

Adjacent JSON files retain source identities, artifact hashes, the schema
check and hosted CI results. No independent disposition, human acceptance,
provider qualification or Warrant completion is asserted.
