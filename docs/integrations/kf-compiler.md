# The Knowledge Fabric compiler interface

What OpenWarrant hands Knowledge Fabric (KF), and what it gets back.
Delivered under OW-WAR-0128. The decision it rests on, and the alternatives,
are [OW-ADR-0027](../adr/atoms/OW-ADR-0027-kf-compiler-interface.md).

## Status: proposed, and not decided

This document is a proposal. **Neither side implements it**, and the three
questions that decide its shape have no recorded answer (OW-WAR-0128,
`20-basis.md`, "Questions for the owners"; `war questions OW-WAR-0128`
reports none answered; `rationale.toml` carries an empty `judgment_ref` for
each):

- **Q-001**, who runs the OpenWarrant compiler (the owner and KF's owner);
- **Q-002**, which component is "the Knowledge Fabric Compiler" (KF's owner);
- **Q-003**, whether KF may hold authored atom bytes to compile them (KF's
  owner).

OW-WAR-0128's work order says the performer drafts after those answers. This
draft was written before them, at the owner's request, so the answers have a
concrete text to accept, amend or reject. It is written **under the
recommended answers** (Q-001 (c), Q-002 (a), Q-003 (a), argued in
OW-ADR-0027); the section "Who invokes whom" says what changes under the
others. Every element below says who owns it and whether it exists:

- an OpenWarrant element names the file it comes from, or says **to build**;
- a KF element cites `KF@3d3c871e:<path>`, the file at that path in the
  Knowledge Fabric repository at commit
  `3d3c871e44e9c62d36ed2d70eaeb46a9a86c9aed` (the commit OW-WAR-0128's basis
  names). KF is another repository, so these are code spans, not links. A KF
  behaviour without such a citation is in "Open questions for KF" and
  nowhere else.

Protocol names `oh.war/compilation-request/v1` and
`oh.war/compilation-result/v1` are proposed (OW-WAR-0128 U-004); the owner
may rename them.

## What exists today

OpenWarrant:

- `war compile` discovers a repository and writes projections into it
  ([compile.rs](../../crates/openwarrant-cli/src/compile.rs)). §83.3 rules
  that out for a KF-invoked process: no ambient source discovery, supplied
  bytes only.
- Underneath it, `lower(&CompilationBasis, &ValidatedManifest)` is pure: "no
  clock, no environment, no filesystem", over manifest and atom bytes held in
  memory ([lower.rs](../../crates/openwarrant-compiler/src/lower.rs)). That
  function is what a §81 compile wraps. `canonical_json` renders the IR and
  `full_warrant` renders `WAR.md`
  ([render.rs](../../crates/openwarrant-compiler/src/render.rs));
  `WarIr::contract_digest` computes the contract digest
  ([ir.rs](../../crates/openwarrant-compiler/src/ir.rs)).
- `war sdk` is an offline JSON-in, JSON-out shell with a 4 MiB input bound,
  a 16 MiB output bound, a bounded JSON decoder (65,536 nodes, depth 64,
  duplicate members refused), and an `oh.war/report/v1` envelope on every
  outcome ([sdk.rs](../../crates/openwarrant-cli/src/sdk.rs),
  [sdk/wire.rs](../../crates/openwarrant-cli/src/sdk/wire.rs)). None of its
  operations is a §81 compile.

Knowledge Fabric:

- KF records the contract digest, Compilation Basis and canonical IR "as
  OpenWarrant computed them", does not recompute, and never stores authored
  atoms (`KF@3d3c871e:docs/decisions/0019-warrants-as-institutional-record.md`,
  "Contract revisions are immutable snapshots"). `submit_warrant` takes
  `contract_digest` and `compilation_basis` as 64-hex sha256 strings with no
  domain, and `canonical_ir` as an object
  (`KF@3d3c871e:packages/warrants/src/index.ts`, `submitWarrant`).
- KF's compiler runtime is a document compiler speaking `kf-document-v1`
  (`KF@3d3c871e:docs/decisions/0002-liminal-backed-document-compiler.md`);
  v1.0 ships no Liminal compiler, and the pinned process adapter is "built
  and tested, wired to nothing"
  (`KF@3d3c871e:docs/decisions/0010-liminal-compiler-deferred.md`).
- That adapter already does what §83.2 and §83.3 ask of a process seam: it
  refuses an executable or `Cargo.lock` whose digest differs from the pinned
  one (`KF@3d3c871e:packages/documents/src/liminal-adapter/executable.ts`),
  runs the process under bubblewrap with `--unshare-all` and `--clearenv`
  (`KF@3d3c871e:packages/documents/src/liminal-adapter/sandbox.ts`), writes
  one canonical JSON request plus a newline and passes `--protocol <name>`
  (`KF@3d3c871e:packages/documents/src/liminal-adapter/compiler-io.ts`), and
  bounds input at 16 MiB, output at 64 MiB and diagnostics at 1 MiB by
  default (`KF@3d3c871e:packages/documents/src/liminal-adapter/limits.ts`).
  The worker refuses an oversize request as `input_size_limit_exceeded`
  (`KF@3d3c871e:apps/worker/src/compiler-runtime/input-guard.ts`).
- No KF code at `3d3c871e` starts an OpenWarrant process.

## Who invokes whom

Under the recommendation (Q-001 (c), Q-002 (a), Q-003 (a)); not decided.

1. **The repository compiles and submits**, as KF ADR 0019 built it: the
   repository side runs `war compile`, and a human or tool posts
   `submit_warrant` with the contract digest, Compilation Basis and
   canonical IR (`KF@3d3c871e:packages/warrants/src/index.ts`). Unchanged
   by this interface.
2. **KF's compiler runtime checks it.** Through an OpenWarrant adapter
   beside the Liminal one (to build, KF side), KF fetches the authored bytes
   from the Source Holder at the named commit, holds them for one
   compilation and stores none of them (Q-003 (a)), invokes the pinned `war`
   binary under this process contract, and compares the result's
   `semantic_digest` with the submitted `contract_digest`. A mismatch is
   KF's refusal to record the submission; its rule name is KF's to choose.
3. **The consuming workflow** reads KF's record, never the process output
   directly (the product spec's "Knowledge Fabric Compiler → OpenWarrant
   compiler → Knowledge Fabric Compiler → consuming workflow",
   [product spec](../design/openwarrant-product-spec.md)).

Under the other answers:

- **Q-001 (a)**, the repository compiles and KF only records: step 2 does not
  exist, and the v1 interface *is* the `submit_warrant` payload above. The
  process contract below is then not needed until a later answer brings it
  back.
- **Q-001 (b)**, KF compiles: step 1 disappears and KF submits what step 2
  returns. The process contract is unchanged; KF ADR 0019's "KF records what
  OpenWarrant computed" then means "what the pinned binary computed".
- **Q-002 (b) or (c)**: the caller in step 2 is a new KF component or an
  outside workflow. The process contract is unchanged; the pins and sandbox
  are then that caller's to enforce.
- **Q-003 (b)**: KF stores the bytes. Nothing here changes; KF ADR 0019's
  "authored atoms never enter this database" does. **Q-003 (c)**: only
  Q-001 (a) remains open.

## The request, `oh.war/compilation-request/v1`

One JSON object. On the wire it is the RFC 8785 (JCS) serialization plus one
newline, as §83.1 ("through canonical JSON") and KF's own adapter do
(`KF@3d3c871e:packages/documents/src/liminal-adapter/compiler-io.ts`).

| §81.1 input | v1 disposition | field and shape | OpenWarrant side | KF side |
|---|---|---|---|---|
| protocol | **carried** | `protocol`: exactly `oh.war/compilation-request/v1` | to build | to build |
| Source Holder snapshots | **carried**, recorded and not fetched | `source_holder`: `kind` (`git` only in v1), `repository`, `commit` (a digest object, below), `path` of the Warrant directory. The compiler never reads it back from anywhere; it echoes it into diagnostics and checks nothing against Git | to build; `kind` values from §13 | to build (KF fetches the bytes before invoking; open question KF-1) |
| manifest | **carried** | `manifest`: `path` (repository-relative, as `manifest_source` in the IR), `text` (the exact bytes, UTF-8), optional `digest` | to build; parsed by `openwarrant_core::Manifest`, validated as `war check` validates it ([manifest.rs](../../crates/openwarrant-core/src/manifest.rs)) | to build |
| supplied source bytes | **carried** | `sources[]`: one entry per manifest atom with `path` (as the manifest writes it), `text`, optional `digest`. Bytes only, never a location | to build; becomes `CompilationBasis::atoms` ([lower.rs](../../crates/openwarrant-compiler/src/lower.rs)) | to build |
| bound records | **deferred**: `lower` reads no bound record, and none enters the contract digest (`WarIr::contract_digest`, [ir.rs](../../crates/openwarrant-compiler/src/ir.rs)). Authorization, verification and resolution records are KF's after registration (§11) | `bound_records` present: refused, `compile.field-deferred` | none | none |
| Workspace Basis | **deferred** as an input: v1 computes the Workspace Basis digest from the supplied bytes and returns it; accepting a caller-built Basis is Liminal's model (§11 table, §11.3) and waits for the Liminal adapter (§82.2) | `workspace_basis` present: refused, `compile.field-deferred` | computed today in `lower` (`workspace_basis_digest`) | none |
| policies | **deferred**: no policy changes what `lower` produces; repository policy governs `war check`, which v1 does not run | `policies` present: refused, `compile.field-deferred` | none | none |
| requested targets | **carried** | `targets[]`: `canonical-ir` (always produced, may be omitted) and `war-md`. Any other value: refused, `compile.target-unsupported`. Corpus views (overviews, `CURRENT.md`) need every Warrant and are not single-Warrant targets | `canonical_json`, `full_warrant` ([render.rs](../../crates/openwarrant-compiler/src/render.rs)); the request surface to build | to build |
| schema pack | **carried** as a pin check | `schema_pack.version`: must equal the pack the binary was built with (`SCHEMA_PACK_VERSION`, today `0.2.0`, [ir.rs](../../crates/openwarrant-compiler/src/ir.rs)); otherwise refused, `compile.schema-pack-mismatch`. The binary carries its pack (`schema_pack_version`, [lower.rs](../../crates/openwarrant-compiler/src/lower.rs)); a caller cannot supply another | exists (the pin); the request check to build | to build |

Optional in v1, outside §81.1's list: `sas` (`version` and `sha256` of the
SAS document), which becomes `CompilationBasis::sas`
([lower.rs](../../crates/openwarrant-compiler/src/lower.rs)); absent means
absent, not empty, as the IR treats it.

Any other top-level field is refused as `compile.field-unknown` (§69.4:
unknown required extensions fail closed; v1 declares no optional
extension).

## The result, `oh.war/compilation-result/v1`

Written to stdout on exit status 0 or 2 (below).

| §81.2 field | v1 disposition | field | OpenWarrant side | KF side |
|---|---|---|---|---|
| canonical WAR IR | **carried** | `canonical_ir`: the IR object `war compile` writes as `WAR.json` | exists: `lower`, `canonical_json` | KF stores it as the §83.5 snapshot (`KF@3d3c871e:packages/warrants/src/index.ts`, `canonical_ir`) |
| semantic digest | **carried**, as the contract digest (proposed mapping: the digest over identity, composition and relations that excludes renderings, §91.1 test 3) | `semantic_digest` | exists: `WarIr::contract_digest` ([ir.rs](../../crates/openwarrant-compiler/src/ir.rs)) | compared with the submitted `contract_digest` (to build) |
| dependency digest | **carried**, as the Workspace Basis digest (proposed mapping: the digest over the format basis and every source digest) | `dependency_digest` | exists: `workspace_basis_digest` in `lower`; generated files already label it "Compilation basis" (`generated_header`, [render.rs](../../crates/openwarrant-compiler/src/render.rs)) | KF's `compilation_basis` field (`KF@3d3c871e:packages/warrants/src/index.ts`) |
| diagnostics | **carried** | `diagnostics[]`: `severity`, `rule`, `file`, `message`, the `oh.war/report/v1` diagnostic shape | the shape exists ([output.rs](../../crates/openwarrant-cli/src/output.rs)); v1 emits manifest validation and lowering findings only, not `war check`'s corpus rules (to build) | to build |
| unresolved refs | **carried** | `unresolved_refs[]`: every `war://`, `adr://`, `sas://` and `roadmap://` reference in the IR's `relations`, since none is supplied and v1 resolves none | to build; the refs exist in `relations` ([lower.rs](../../crates/openwarrant-compiler/src/lower.rs)) | resolution through federation is KF's (§11.1, RQ-005) |
| omitted subgraphs | **carried** | `omitted_subgraphs[]`: JSON pointers of IR sections left out, today `/execution`, `/assurance_case`, `/resolution` ("Absent is not empty", [ir.rs](../../crates/openwarrant-compiler/src/ir.rs)) | to build (the list); the omission exists | to build |
| source maps | **deferred**: the Markdown v1 adapter produces none; source maps are Liminal's (§11 table) and arrive with the Liminal adapter (§82.2) | absent | none | none |
| conversion loss | **deferred**: v1 converts nothing between Holders or formats; the Markdown adapter reads authored atoms as they are. Absent, never an empty list that would claim "no loss measured" | absent | none | none |
| compiled views | **carried** for `war-md` when requested | `views`: `{ "war-md": { "text", "digest" } }` | exists: `full_warrant` | to build |

Also carried: `digest_index[]`, one entry per digest inside `canonical_ir`,
giving its JSON pointer, algorithm and domain (next section). To build.

## Digests

Every digest crossing this interface is an object
`{ "algorithm", "domain", "hex" }` (§65.1, RQ-081). None is a bare string.

| digest | algorithm | domain | computed over | source |
|---|---|---|---|---|
| `semantic_digest` (contract) | `sha256` | `oh.war/contract/v1` | the §65.2 preimage of coverage, format basis, identity, composition, relations | `WarIr::contract_digest`, [ir.rs](../../crates/openwarrant-compiler/src/ir.rs) |
| `dependency_digest` (Workspace Basis) | `sha256` | `oh.war/workspace-basis/v1` | the §65.2 preimage of format basis and composition | `lower`, [lower.rs](../../crates/openwarrant-compiler/src/lower.rs) |
| composition revision (in `digest_index`) | `sha256` | `oh.war/composition-revision/v1` | the §65.2 preimage of the atom list and scope | `lower` |
| manifest and each atom source (request `digest`, result `digest_index`) | `sha256` | `raw-bytes` | the exact bytes, **no** §65.2 preimage | `sha256_hex`, [digest.rs](../../crates/openwarrant-compiler/src/digest.rs); used by `lower` |
| a view's text (`views.*.digest`) | `sha256` | `raw-bytes` | the exact rendered bytes | to build |
| `source_holder.commit` | `sha1` (or `sha256` for a SHA-256 Git repository) | `git-commit` | Git's object id | Git |

`raw-bytes` is a label this interface proposes, not a §65 domain: `lower`
hashes manifest and atom bytes directly, while §65 lists
`atom_source_digest` and `manifest_digest` among its domains. Whether those
two should move to §65.2 preimages is open question OW-1; moving them would
change every contract digest (§69.3). Inside `canonical_ir` the IR's own
digest fields stay bare hex, with the algorithm stated once in
`integrity.algorithm`; the IR format is frozen for OW-WAR-0128, so
`digest_index` is how the result names each one's domain.

## The process contract

**Invocation** (to build): `war compile --protocol oh.war/compilation-request/v1`,
request on stdin. The `--protocol` convention is the one KF's adapter already
passes (`KF@3d3c871e:packages/documents/src/liminal-adapter/compiler-io.ts`)
and §75.2's process seam uses. In this mode `war`:

- reads exactly one request from stdin and nothing else: it opens no file,
  reads no repository, and resolves no `openwarrant.toml`;
- makes no network connection: nothing in the request is fetched;
- writes one JSON document and a newline to stdout, and bounded diagnostics
  to stderr;
- produces output that depends on the request bytes and the binary alone
  (`lower` is pure, [lower.rs](../../crates/openwarrant-compiler/src/lower.rs)).

These are properties of the binary, each with a refusal or a plant that
observes it (to build). The sandbox around it (§83.3) is the caller's: KF's
adapter pattern runs a pinned process under bubblewrap with no network and a
cleared environment (`KF@3d3c871e:packages/documents/src/liminal-adapter/sandbox.ts`).
The binary does not rely on it: every refusal below fires with or without a
sandbox.

**Bounds** (to build; the numbers follow `war sdk`'s and are the owner's to
change):

| bound | value | on exceeding it |
|---|---|---|
| stdin | 4 MiB, counted before parsing | refused, `compile.input-limit` |
| JSON shape | 65,536 nodes, depth 64, duplicate members refused (the `sdk/wire.rs` decoder) | refused, `compile.request-malformed` |
| stdout | 16 MiB | refused, `compile.output-limit`; nothing partial is written |
| stderr | 1 MiB (KF's adapter default for diagnostics) | truncated with a final line saying so; stdout is unaffected |

**Exit status.** The codes are the ones `war` already uses
([lib.rs](../../crates/openwarrant-cli/src/lib.rs): `EXIT_OK`,
`EXIT_DIAGNOSTIC`, `EXIT_NOT_READY`), with the meaning `war sdk` gives a
refusal:

| status | meaning | stdout |
|---|---|---|
| 0 | compiled | an `oh.war/compilation-result/v1` with `canonical_ir` |
| 1 | the request was refused; nothing was compiled | an `oh.war/report/v1` envelope with one `error` diagnostic whose `rule` is the code below, and `result: null` |
| 2 | the request was sound and the Warrant did not compile (an invalid manifest, a missing required atom) | an `oh.war/compilation-result/v1` with `diagnostics` and no `canonical_ir` |

**Refusals.** Each writes nothing but the envelope, exits 1, and is checked
in this order, so a request with several faults names the first:

| # | the input that triggers it | error code | exit |
|---|---|---|---|
| 1 | more than 4 MiB on stdin | `compile.input-limit` | 1 |
| 2 | stdin that is not one JSON object within the shape bounds, or has a duplicate member | `compile.request-malformed` | 1 |
| 3 | request bytes that differ from their own RFC 8785 serialization plus one newline | `compile.request-not-canonical` | 1 |
| 4 | a `protocol` other than `oh.war/compilation-request/v1`, or a `--protocol` argument naming another | `compile.protocol-unsupported` | 1 |
| 5 | `--root`, or any argument naming a directory, given with `--protocol` (ambient source discovery, §83.3) | `compile.ambient-source` | 1 |
| 6 | a top-level field v1 does not define | `compile.field-unknown` | 1 |
| 7 | `bound_records`, `workspace_basis` or `policies` present | `compile.field-deferred` | 1 |
| 8 | a path given instead of bytes: a `manifest` or `sources[]` entry with `path` and no `text`, or a manifest atom with no `sources[]` entry for its path | `compile.path-not-bytes` | 1 |
| 9 | a request needing the network: a `url`, `uri` or `href` field anywhere in `manifest`, `sources[]` or `source_holder`, a `text` given as a `file:`, `http:` or `https:` URL, or a manifest atom declared by `ref` (an `atom://` reference, §15.5) with no `sources[]` entry for that ref | `compile.network-required` | 1 |
| 10 | `text` that is not valid UTF-8 | `compile.source-not-utf8` | 1 |
| 11 | a supplied `digest` that does not match the bytes, or names an algorithm or domain other than those in the digest table | `compile.digest-mismatch` | 1 |
| 12 | `schema_pack.version` other than the binary's pack | `compile.schema-pack-mismatch` | 1 |
| 13 | a `targets` value other than `canonical-ir` or `war-md` | `compile.target-unsupported` | 1 |
| 14 | a result larger than 16 MiB | `compile.output-limit` | 1 |

## Pins the caller records (§83.2)

Each is a SHOULD in §83.2. KF records them with every invocation; the
OpenWarrant side says where each comes from.

| pin | value | where it comes from | status |
|---|---|---|---|
| OpenWarrant commit | a 40-hex Git object id, a `git-commit` digest object | `war --version` reports it for a non-release build ([build_identity.rs](../../crates/openwarrant-cli/src/build_identity.rs)); a release names its tag | exists (reported); recording is KF's, to build |
| `Cargo.lock` digest | `sha256`, `raw-bytes` | the file at that commit | KF's adapter pattern checks it (`KF@3d3c871e:packages/documents/src/liminal-adapter/executable.ts`); for `war`, to build |
| executable digest | `sha256`, `raw-bytes` | the binary KF runs | same citation; for `war`, to build |
| runtime closure digest | `sha256` over the sorted list of runtime files and their digests | the caller's sandbox (KF's `runtimeFilePaths`, `KF@3d3c871e:packages/documents/src/liminal-adapter/contracts.ts`) | KF, to build |
| protocol version | `oh.war/compilation-request/v1` | the request | to build |
| qualification receipt | a reference to the receipt that qualified this binary for this protocol | **deferred**: no qualification process for `war` as a KF compiler exists on either side | open (KF-3) |

## Examples

Produced, not invented. A scratch repository was initialized with this
build's `war init --namespace EX`, given one minimal `delivery` Warrant
(`EX-WAR-0001`, five one-line atoms), committed as
`fdf1d07f15f1ba4df403f06c802431b9e9629555`, and compiled with `war compile`.
The request below carries that commit's exact manifest and atom bytes. The
result's `canonical_ir` is the `WAR.json` `war compile` wrote; its digests are
the ones `war compile` and `war authorize --json` (the contract digest)
reported, and each atom's `raw-bytes` digest was recomputed with `sha256`
over the file and matched. No part of the result was produced by the
interface itself, which does not exist yet. Shown indented for reading; the
wire form is canonical.

### A request, and its result

The request (`EX-WAR-0001` at `fdf1d07f`, target `canonical-ir`):

```json
{
  "protocol": "oh.war/compilation-request/v1",
  "source_holder": {
    "kind": "git",
    "repository": "kfex",
    "commit": {
      "algorithm": "sha1",
      "domain": "git-commit",
      "hex": "fdf1d07f15f1ba4df403f06c802431b9e9629555"
    },
    "path": "docs/warrants/EX-WAR-0001"
  },
  "manifest": {
    "path": "docs/warrants/EX-WAR-0001/manifest.toml",
    "text": "schema = \"oh.war/manifest/v1\"\nuuid = \"01a0e50b-73f7-7333-ac2b-c2c6487c68f6\"\nlocal_alias = \"EX-WAR-0001\"\nenterprise_id = \"\"\ntitle = \"Add a changelog\"\nprofile = \"delivery\"\nassurance_level = \"basic\"\n\n[[atoms]]\nordinal = 10\nrole = \"intent\"\npath = \"atoms/10-intent.md\"\nrequired = true\n\n[[atoms]]\nordinal = 20\nrole = \"basis\"\npath = \"atoms/20-basis.md\"\nrequired = true\n\n[[atoms]]\nordinal = 40\nrole = \"work_order\"\npath = \"atoms/40-work-order.md\"\nrequired = true\n\n[[atoms]]\nordinal = 45\nrole = \"milestones\"\npath = \"atoms/45-milestones.yaml\"\nrequired = true\n\n[[atoms]]\nordinal = 60\nrole = \"assurance\"\npath = \"atoms/60-assurance.md\"\nrequired = true\n",
    "digest": {
      "algorithm": "sha256",
      "domain": "raw-bytes",
      "hex": "e40437e7cddcd1018db1d4bdb54d1f88a02f0d22219899097616ff76e016cc97"
    }
  },
  "sources": [
    {
      "path": "atoms/10-intent.md",
      "text": "---\nschema: oh.war/atom/v1\nwarrant_uuid: 01a0e50b-73f7-7333-ac2b-c2c6487c68f6\nrole: intent\njurisdiction: authored\norder: 10\nclassification: internal\n---\n\n# Intent\n\nAdd CHANGELOG.md.\n",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "f47ec8a32d3d20c8c3336869f4b459fe36607d09ddfeda13dbe59ca67fdb7a57"
      }
    },
    {
      "path": "atoms/20-basis.md",
      "text": "---\nschema: oh.war/atom/v1\nwarrant_uuid: 01a0e50b-73f7-7333-ac2b-c2c6487c68f6\nrole: basis\njurisdiction: authored\norder: 20\nclassification: internal\n---\n\n# Basis\n\nKeep a Changelog 1.1.0.\n",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "6b6c5a3274142df3fd5369bb4555d40fa33a7bceac74630d55e33957b5aea34d"
      }
    },
    {
      "path": "atoms/40-work-order.md",
      "text": "---\nschema: oh.war/atom/v1\nwarrant_uuid: 01a0e50b-73f7-7333-ac2b-c2c6487c68f6\nrole: work_order\njurisdiction: authored\norder: 40\nclassification: internal\n---\n\n# Work Order\n\nDeliver CHANGELOG.md.\n",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "788fabe5a0cd89b16a7f25f2d650e3d1fdddfd2468e5f3b70e0e0eab286fcba1"
      }
    },
    {
      "path": "atoms/45-milestones.yaml",
      "text": "schema: \"oh.war/milestones/v1\"\n\nmilestones: []\nstages: []\n",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "024a9866e573137a321d6119ae15e787df9903e198fcdc5d9b892e10d477538a"
      }
    },
    {
      "path": "atoms/60-assurance.md",
      "text": "---\nschema: oh.war/atom/v1\nwarrant_uuid: 01a0e50b-73f7-7333-ac2b-c2c6487c68f6\nrole: assurance\njurisdiction: authored\norder: 60\nclassification: internal\n---\n\n# Assurance\n\nOBL-001: CHANGELOG.md exists.\n",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "e0e0ccc268ea10c0b34dd0d32a06bb6c71e37b06a7472dc6a46030bdaab1b11c"
      }
    }
  ],
  "schema_pack": {
    "version": "0.2.0"
  },
  "targets": [
    "canonical-ir"
  ]
}
```

Its result, exit status 0:

```json
{
  "protocol": "oh.war/compilation-result/v1",
  "canonical_ir": {
    "api_version": "oh.war/v1",
    "contract_coverage": {
      "covered": [
        "intent",
        "scope",
        "basis_requirements",
        "assumptions",
        "constraints",
        "adr_references",
        "milestones",
        "stages"
      ]
    },
    "contract_revision": 1,
    "format_basis": {
      "package_id": "openwarrant-schema-pack",
      "profile_schema_id": "delivery",
      "root_schema_id": "work_authorization_record",
      "version": "0.2.0"
    },
    "identity": {
      "assurance_level": "basic",
      "local_alias": "EX-WAR-0001",
      "profile": "delivery",
      "title": "Add a changelog",
      "uuid": "01a0e50b-73f7-7333-ac2b-c2c6487c68f6"
    },
    "integrity": {
      "algorithm": "sha256",
      "composition_revision_digest": "3d72fa279803e61f897ddb7bff35b607112e77f634a9a443dd4d1df373594c47",
      "workspace_basis_digest": "c171d3959a38615ef8536f4c34cd4aad9ef3022288bb9e4c53bbbf09594647ef"
    },
    "kind": "work_authorization_record",
    "relations": {},
    "source_and_composition": {
      "atoms": [
        {
          "atom_source_digest": "f47ec8a32d3d20c8c3336869f4b459fe36607d09ddfeda13dbe59ca67fdb7a57",
          "jurisdiction": "authored",
          "ordinal": 10,
          "required": true,
          "role": "intent",
          "source": "atoms/10-intent.md"
        },
        {
          "atom_source_digest": "6b6c5a3274142df3fd5369bb4555d40fa33a7bceac74630d55e33957b5aea34d",
          "jurisdiction": "authored",
          "ordinal": 20,
          "required": true,
          "role": "basis",
          "source": "atoms/20-basis.md"
        },
        {
          "atom_source_digest": "788fabe5a0cd89b16a7f25f2d650e3d1fdddfd2468e5f3b70e0e0eab286fcba1",
          "jurisdiction": "authored",
          "ordinal": 40,
          "required": true,
          "role": "work_order",
          "source": "atoms/40-work-order.md"
        },
        {
          "atom_source_digest": "024a9866e573137a321d6119ae15e787df9903e198fcdc5d9b892e10d477538a",
          "jurisdiction": "authored",
          "ordinal": 45,
          "required": true,
          "role": "milestones",
          "source": "atoms/45-milestones.yaml"
        },
        {
          "atom_source_digest": "e0e0ccc268ea10c0b34dd0d32a06bb6c71e37b06a7472dc6a46030bdaab1b11c",
          "jurisdiction": "authored",
          "ordinal": 60,
          "required": true,
          "role": "assurance",
          "source": "atoms/60-assurance.md"
        }
      ],
      "manifest_digest": "e40437e7cddcd1018db1d4bdb54d1f88a02f0d22219899097616ff76e016cc97",
      "manifest_source": "docs/warrants/EX-WAR-0001/manifest.toml"
    }
  },
  "semantic_digest": {
    "algorithm": "sha256",
    "domain": "oh.war/contract/v1",
    "hex": "2c823f0ae49aad98daaf27fa38293b95a44803000c863d7302cb61ed78ea08c8"
  },
  "dependency_digest": {
    "algorithm": "sha256",
    "domain": "oh.war/workspace-basis/v1",
    "hex": "c171d3959a38615ef8536f4c34cd4aad9ef3022288bb9e4c53bbbf09594647ef"
  },
  "digest_index": [
    {
      "pointer": "/source_and_composition/manifest_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "e40437e7cddcd1018db1d4bdb54d1f88a02f0d22219899097616ff76e016cc97"
      }
    },
    {
      "pointer": "/source_and_composition/atoms/0/atom_source_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "f47ec8a32d3d20c8c3336869f4b459fe36607d09ddfeda13dbe59ca67fdb7a57"
      }
    },
    {
      "pointer": "/source_and_composition/atoms/1/atom_source_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "6b6c5a3274142df3fd5369bb4555d40fa33a7bceac74630d55e33957b5aea34d"
      }
    },
    {
      "pointer": "/source_and_composition/atoms/2/atom_source_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "788fabe5a0cd89b16a7f25f2d650e3d1fdddfd2468e5f3b70e0e0eab286fcba1"
      }
    },
    {
      "pointer": "/source_and_composition/atoms/3/atom_source_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "024a9866e573137a321d6119ae15e787df9903e198fcdc5d9b892e10d477538a"
      }
    },
    {
      "pointer": "/source_and_composition/atoms/4/atom_source_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "raw-bytes",
        "hex": "e0e0ccc268ea10c0b34dd0d32a06bb6c71e37b06a7472dc6a46030bdaab1b11c"
      }
    },
    {
      "pointer": "/integrity/composition_revision_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "oh.war/composition-revision/v1",
        "hex": "3d72fa279803e61f897ddb7bff35b607112e77f634a9a443dd4d1df373594c47"
      }
    },
    {
      "pointer": "/integrity/workspace_basis_digest",
      "digest": {
        "algorithm": "sha256",
        "domain": "oh.war/workspace-basis/v1",
        "hex": "c171d3959a38615ef8536f4c34cd4aad9ef3022288bb9e4c53bbbf09594647ef"
      }
    }
  ],
  "diagnostics": [],
  "unresolved_refs": [],
  "omitted_subgraphs": [
    "/execution",
    "/assurance_case",
    "/resolution"
  ],
  "views": {}
}
```

### A refused request, and its error

The same request with `protocol` set to a version this build does not answer,
and its manifest given by path with no bytes. Refusal 4 is checked before
refusal 8, so the protocol is what the error names:

```json
{
  "protocol": "oh.war/compilation-request/v2",
  "source_holder": {
    "kind": "git",
    "repository": "kfex",
    "commit": {"algorithm": "sha1", "domain": "git-commit", "hex": "fdf1d07f15f1ba4df403f06c802431b9e9629555"},
    "path": "docs/warrants/EX-WAR-0001"
  },
  "manifest": {"path": "docs/warrants/EX-WAR-0001/manifest.toml"},
  "sources": [],
  "schema_pack": {"version": "0.2.0"},
  "targets": ["canonical-ir"]
}
```

Exit status 1, and on stdout:

```json
{
  "schema": "oh.war/report/v1",
  "command": "compile",
  "diagnostics": [
    {
      "severity": "error",
      "rule": "compile.protocol-unsupported",
      "file": null,
      "message": "protocol \"oh.war/compilation-request/v2\" is not one this build answers; it answers oh.war/compilation-request/v1"
    }
  ],
  "notes": [],
  "counts": {"pass": 0, "warn": 0, "unknown": 0, "error": 1, "worst": "error"},
  "verdict": "not_ready",
  "verdict_line": "compilation request refused",
  "exit_code": 1,
  "result": null
}
```

`compile.protocol-unsupported` is refusal 4 of the process contract.

## Open questions for KF

- **KF-1.** Does KF's worker fetch Warrant bytes from a Git Source Holder at a
  named commit? Its compiler inputs at `3d3c871e` are object-store references
  (`KF@3d3c871e:apps/worker/src/compiler-runtime/input-guard.ts`,
  `loadCompilerInputs`); a Git fetch path was not found and is not claimed.
- **KF-2.** Would the OpenWarrant adapter implement KF's
  `DocumentCompilerAdapter`, or a Warrant-specific interface? The existing one
  is shaped for `kf-document-v1`
  (`KF@3d3c871e:docs/decisions/0002-liminal-backed-document-compiler.md`).
- **KF-3.** What qualifies a `war` binary for this protocol, and who issues
  the receipt §83.2 asks KF to pin?
- **KF-4.** `submit_warrant` takes `contract_digest` and `compilation_basis`
  as bare 64-hex strings (`KF@3d3c871e:packages/warrants/src/index.ts`,
  `sha()`). RQ-081 asks cross-system digests to name algorithm and domain.
  Does KF accept the digest objects above, or record the domain beside the
  hex?
- **KF-5.** Under Q-001 (c), what does KF do on a digest mismatch, and what
  does it record?

## Open questions for the owner

- **OW-1.** `lower` hashes manifest and atom bytes with no §65.2 preimage.
  Keep `raw-bytes` as a declared domain for them, or move them to preimages
  in a major protocol version (§69.3)?
- **OW-2.** The bounds (4 MiB in, 16 MiB out, 1 MiB diagnostics) follow
  `war sdk` and KF's adapter. KF's adapter defaults to 16 MiB in
  (`KF@3d3c871e:packages/documents/src/liminal-adapter/limits.ts`). Which
  side's number binds?
