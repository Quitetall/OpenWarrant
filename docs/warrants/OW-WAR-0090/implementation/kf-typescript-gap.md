# Knowledge Fabric TypeScript integration gap

Read-only source observation; no provider qualification, signed record change or
accepted reconciliation. OW-WAR-0032 remains incomplete.

Observed sources:

- OpenWarrant main cd4e3985d113b4ab96d84c1610291625e3c09073.
- Knowledge Fabric clean checkout 034e2213873a542fbf04711c1fe2fedb75f10d5c,
  `/mnt/4tb/openhuman-knowledge-fabric`.

OW32 work order requires generated TypeScript types for KF. OBL-004 explicitly
requires generated types to exist and KF to import them. Current OpenWarrant
`crates/openwarrant-cli/src/schemas.rs` generates JSON schemas and `schemas/pack.json`;
its generation path has no TypeScript emitter. KF's `packages/warrants/src/projections.ts`
imports KF actions/database types, then hand-validates incoming record-shaped
payloads. Its `generated/typescript/ontology.ts` is KF ontology output, not evidence
that KF imports generated OpenWarrant record types. These source observations do
not satisfy OW32 OBL-004.

Next shared scope must cover both repositories in one contract:

1. Generate deterministic TypeScript declarations from the actual OpenWarrant
   source schemas/types, with explicit supported schema-draft semantics and
   refusal for unsupported constructs. Do not replace inconvenient types with any.
2. Pin generated artifact and source identities. Preserve existing schema-pack
   identity/digest semantics; any necessary wire change needs its decision first.
3. Have KF import the generated declarations at its actual Warrant boundary.
   Keep runtime validation: TypeScript erases at runtime and cannot authorize data.
4. Compile real positive and negative consumer fixtures, observe schema/type drift
   refusal, and prove the imports belong to KF's normal build path.
5. Retain real build versions and checks from each participant. A standalone demo
   file, invented success receipt or copied handwritten interface is insufficient.

No source implementation was changed during this observation. Other KF server
obligations (typed actions, concurrency, direct-write refusal and Source Holder
behavior) remain separate requirements and cannot be closed by TypeScript imports.
