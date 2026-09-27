---
schema: oh.war/atom/v1
adr_uuid: 01a0e507-828f-7727-9c09-678a8e680fb9
local_alias: OW-ADR-0027
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0d04c-5f6e-7d90-8eed-cf0953b77657"
---

# ADR OW-0027: The Knowledge Fabric compiler interface — who compiles, and the process contract between them

## Status

Proposed by the performer under OW-WAR-0128. **Not decided.** The three
blocking questions of that Warrant (`20-basis.md`, "Questions for the
owners") have no recorded answer: `war questions OW-WAR-0128` reports none
answered, and `rationale.toml` carries an empty `judgment_ref` for each.

- **Q-001**, who runs the OpenWarrant compiler: the owner **and** KF's owner;
- **Q-002**, which component is "the Knowledge Fabric Compiler": KF's owner;
- **Q-003**, whether KF may hold authored atom bytes to compile them: KF's
  owner.

OW-WAR-0128's work order says the performer drafts after those answers. This
draft was written before them, at the owner's request, so the answers have a
concrete text to accept, amend or reject. Each question below states the
options, recommends one, and says it is not decided. The interface,
[kf-compiler.md](../../integrations/kf-compiler.md), is written under the
recommended answers and says what changes under the others. When the owners
answer, the Decision section is rewritten to the answers, the options not
taken stay as the record of the choice, and status moves to `accepted` by
the owner's act, not the performer's. Two of the three questions are KF's
owner's: acceptance of this ADR in this repository does not answer them for
Knowledge Fabric.

Knowledge Fabric is cited as `KF@3d3c871e:<path>`, the file at that path in
the Knowledge Fabric repository at commit
`3d3c871e44e9c62d36ed2d70eaeb46a9a86c9aed`. A KF behaviour without such a
citation appears only as a question for KF's owner, in
[kf-compiler.md](../../integrations/kf-compiler.md) ("Open questions for
KF").

## Context

- The product spec names the composition "Knowledge Fabric Compiler →
  OpenWarrant compiler → Knowledge Fabric Compiler → consuming workflow" and
  says the interface and the responsibility for each input and output
  "remain to be designed"
  ([product spec](../../design/openwarrant-product-spec.md)).
- The SAS gives an illustrative §81 `compile(request) -> result` with nine
  inputs (§81.1) and nine outputs (§81.2); §83.1 says KF SHOULD invoke an
  exact pinned OpenWarrant binary through canonical JSON; §83.2 lists pins;
  §83.3 the sandbox; §65 the digest domains; §69 versioning; §75.2 the
  process seam ([the SAS](../../sas/WAR_Software_Architecture_Specification.md)).
- KF records the contract digest, Compilation Basis and canonical IR "as
  OpenWarrant computed them", does not recompute, and never stores authored
  atoms (`KF@3d3c871e:docs/decisions/0019-warrants-as-institutional-record.md`).
  KF's compiler runtime is a document compiler, and v1.0 carries no Liminal
  compiler
  (`KF@3d3c871e:docs/decisions/0002-liminal-backed-document-compiler.md`,
  `KF@3d3c871e:docs/decisions/0010-liminal-compiler-deferred.md`).
- On the OpenWarrant side, `lower` is a pure function of in-memory bytes
  ([lower.rs](../../../crates/openwarrant-compiler/src/lower.rs)); `war compile`
  wraps it with repository discovery and writes
  ([compile.rs](../../../crates/openwarrant-cli/src/compile.rs)); no request or
  result type for §81 exists.

The product spec reads "KF supplies the inputs"; KF ADR 0019 reads "the
repository computes, KF records". This ADR does not reconcile them by
choosing silently; Q-001 is that reconciliation, and it is the owners'.

## Q-001 — who runs the OpenWarrant compiler (not decided)

### (c) Both: the repository compiles, KF recompiles to check — recommended

The repository compiles and submits as KF ADR 0019 built it
(`submit_warrant` with contract digest, Compilation Basis and canonical IR,
`KF@3d3c871e:packages/warrants/src/index.ts`). KF's compiler invokes the
pinned binary over the same bytes and compares digests before recording.

- For: the only option in which both documents hold with one change. The
  record is still what OpenWarrant computed in the repository (ADR 0019),
  and KF supplies inputs and consumes the output (the product spec). KF
  stops recording a digest it cannot check.
- Against: KF ADR 0019's "does not recompute" must become "does not
  recompute as the authority; may recompute to check". That is a KF
  decision, and it needs Q-003 (a) or (b).

### (a) The repository compiles; KF only records

What exists. The v1 "interface" is then the `submit_warrant` payload, and
the §81 process contract is not needed yet. Not recommended: KF records a
contract digest nobody on its side can reproduce, and the product spec's
composition is not met.

### (b) KF compiles over bytes it fetched

§83.1's shape and the product spec's reading. Not recommended alone: KF
becomes the compiler of record, which contradicts ADR 0019's authority
statement, and a repository-side `war compile` then produces a digest that
is not the one recorded. The process contract is the same as under (c).

## Q-002 — what the Knowledge Fabric Compiler is (not decided; KF's owner)

### (a) KF's existing compiler runtime, with an OpenWarrant adapter beside the Liminal one — recommended

KF's pinned Liminal process adapter already refuses a mismatched executable
or `Cargo.lock` digest, sandboxes with bubblewrap (`--unshare-all`,
`--clearenv`), writes one canonical request and passes `--protocol`, and
bounds input, output and diagnostics
(`KF@3d3c871e:packages/documents/src/liminal-adapter/executable.ts`,
`KF@3d3c871e:packages/documents/src/liminal-adapter/sandbox.ts`,
`KF@3d3c871e:packages/documents/src/liminal-adapter/compiler-io.ts`,
`KF@3d3c871e:packages/documents/src/liminal-adapter/limits.ts`). That is most of §83.2
and §83.3 already built once. Whether the OpenWarrant adapter can share the
runtime's `DocumentCompilerAdapter` interface is open (KF-2 in
`kf-compiler.md`).

### (b) A new KF component

Rebuilds the pinning and sandbox that (a) reuses. Not recommended unless
KF's owner finds the document runtime cannot carry a Warrant.

### (c) A workflow outside KF that calls both

Moves pinning and sandboxing outside the system that records the result,
so KF would record what an unpinned caller says it ran. Not recommended.

## Q-003 — may KF hold authored atom bytes (not decided; KF's owner)

### (a) Transiently, for one compilation, never stored — recommended

Keeps ADR 0019's "authored atoms never enter this database" true, and is
what Q-001 (b) or (c) needs.

### (b) Stored, with Git still the Source Holder

Changes ADR 0019, and §11.1 then needs care: a stored copy must not read as
a Holder transfer. Not recommended.

### (c) No

Then only Q-001 (a) remains, and this interface's process contract waits.

## Protocol names (U-004)

`oh.war/compilation-request/v1` and `oh.war/compilation-result/v1`, proposed;
a refusal is an `oh.war/report/v1` envelope, as `war sdk` already emits
([sdk.rs](../../../crates/openwarrant-cli/src/sdk.rs)). The owner may rename
them. A breaking change after acceptance is a major version and an ADR
(§69.3).

## Decision

*Pending Q-001, Q-002 and Q-003.* If the recommendations are taken: Q-001
(c), Q-002 (a), Q-003 (a); the protocol names above; and the interface in
[kf-compiler.md](../../integrations/kf-compiler.md) — every §81.1 input and
§81.2 output dispositioned as carried or deferred with its reason, the
process contract with fourteen named refusals, the §83.2 pins, and every
digest named with algorithm and domain.

## Consequences (if the recommendations are taken)

- OpenWarrant builds `war compile --protocol oh.war/compilation-request/v1`
  over the existing pure `lower`, and plants each refusal by its triggering
  input. No existing record schema or `oh.war/report/v1` changes.
- KF builds an OpenWarrant adapter and the digest comparison, and revises
  ADR 0019's "does not recompute" sentence. Those are KF's Warrants, not
  OpenWarrant's.
- `raw-bytes` digests (manifest and atom sources) stay as `lower` computes
  them; moving them to §65.2 preimages is a separate, breaking decision
  (OW-1 in `kf-compiler.md`).
- R-001 (from OW-WAR-0128): one side builds something else after acceptance.
  Every element of the interface names its owner and whether it exists, so
  the gap shows in review.
- R-002 (from OW-WAR-0128): §81 is illustrative, and a later SAS revision may
  fix another shape. This ADR is then superseded, not edited.

## Rejected alternatives

Listed under each question above: Q-001 (a) and (b), Q-002 (b) and (c),
Q-003 (b) and (c), each with its reason. They are the recommendation's
rejections, not the owners', until the owners answer.
