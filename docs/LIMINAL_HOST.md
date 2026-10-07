# Hosting the WAR domain in Liminal: `oh.war/liminal-v1`

Liminal is the document substrate: Nodes, Relations, Workspace Basis,
Jurisdiction, storage, synchronization and repair. OpenWarrant owns the WAR
domain, and Liminal runs OpenWarrant's compiler instead of re-implementing it
(SAS §11.3; OW-ADR-0031; Liminal SAS §100). Until a plugin interface is
qualified, the boundary is a process: `war host` (SAS §82.2).

```sh
war host < request.json > response.json     # pure: one request in, one response out
war host --export [--projection warrant:*]  # this repository's request (honours --root)
```

Built under OW-WAR-0148 M8 (ticket t-67b5f). The code is
`crates/openwarrant-cli/src/host.rs` and `crates/openwarrant-cli/src/vfs.rs`.
The parity suite is `conformance/host/`.

## The mapping

| OpenWarrant (`oh.war/model/v1`) | Liminal |
|---|---|
| record `{id, type, source, revision, governed_by}` | Node `{id, type, source, revision, jurisdiction}` |
| relation `{from, kind, to, to_revision}` | Relation, unchanged |
| a record's revision, and the file bytes it is read from | a member of the Workspace Basis |
| `governed_by` (who governs a record) | Jurisdiction |
| a profile (`profiles/<name>.toml`) | schema data, sent beside the basis |

## One implementation

`war model` builds the model from a `Corpus` over a `Repository`.
`war host` builds it with the same function (`model::build`) over the same
`Corpus` and `Repository`. The only difference is where the readers' bytes
come from. Every reader on the model's path reads through `crate::vfs`:

- **Outside `war host`**, each `vfs` function is the `std::fs` call it
  replaced. `war model`'s bytes did not change.
- **In a hosted run**, the files are the request's basis, held in memory under
  a virtual root (`/liminal-basis`). A path under that root is answered from
  the basis. Any other path does not exist.

No second interpreter exists. A difference between a hosted and a standalone
model is a difference in input. It is never a second reading of a record.

## The request

```json
{
  "protocol": "oh.war/liminal-v1",
  "version": 1,
  "basis": {
    "id": "liminal-basis://…",
    "digest": "sha256:…",
    "members": [
      {"path": "openwarrant.toml", "digest": "sha256:…", "utf8": "…"},
      {"path": "docs/x.bin", "digest": "sha256:…", "hex": "00ff…"},
      {"path": "docs/warrants/W/gate-runs/run.stdout.txt", "digest": "sha256:…", "withheld": true}
    ]
  },
  "profiles": [{"name": "delivery", "digest": "sha256:…", "utf8": "…"}],
  "nodes": [{"id": "REQ-pr1", "type": "requirement", "source": "…", "revision": "sha256:…"}],
  "relations": [{"from": "REQ-pr1", "kind": "implements", "to": "OUT-pr1"}],
  "observations": {"signatures": [], "git": []},
  "options": {"projections": [{"kind": "warrant", "subject": "*"}], "limits": {"members": 100}}
}
```

- **basis.members** carry exact bytes: `utf8`, or lowercase `hex`. A member
  may instead be `withheld`, known by digest only. Such a member is listed and
  found, but a reader that needs its bytes makes the run not established
  (`host.member-withheld`). `--export` withholds the files the standalone
  build listed and never read, such as gate-run transcripts.
- **basis.digest** is `sha256:` over the JCS of the sorted `[path, digest]`
  pairs. Withheld members are included. The host recomputes it and echoes it.
- **profiles** are mounted at `profiles/<name>.toml`.
- **nodes and relations** are what Liminal holds. They are compared with the
  compiled records and relations, and never used in their place. A
  difference is reported by rule: `host.node-missing`, `host.node-unknown`,
  `host.node-differs`, `host.relation-missing`, `host.relation-unknown`, and
  the `-duplicate` rules.
- **observations** are facts the readers observe outside the bytes (see
  below).
- **options.projections**: `{kind: "warrant", subject: <alias> | "*"}` returns
  a Warrant's committed views (`generated/WAR.md`, `generated/WAR.json`),
  rendered by the same function `war compile` uses.
- **options.limits** may lower any limit. It cannot raise one.

The schemas are `schemas/oh.war/liminal-request/v1.json` and
`liminal-response/v1.json`, with TypeScript types beside them.

## The response

`{protocol, version, outcome, basis{id, digest}, model, model_digest,
diagnostics[], projections[], observations, limits, refusal}`

- `model` is the `oh.war/model/v1` document, with its own diagnostics inside
  it.
- `model_digest` is the sha256 of the model's compact JSON bytes.
- `diagnostics` holds the host's own findings, each with severity `error` or
  `unknown`.
- `observations` counts the observations supplied, used and missing, and
  carries `authenticated: false`.

**Exit codes:**

- **0:** compiled. The Nodes and Relations are the compiled ones, every
  projection rendered, and every observation the run needed was supplied.
- **1:** refused. No model is returned, and `refusal` names the rule.
- **2:** compiled, but something is not established. A Node or Relation
  differs, a projection is unavailable, an observation was missing, a
  withheld member was needed, or the basis is no repository
  (`host.not-compiled`).

`war sdk` has 0 and 1 only. Code 2 is the one addition.

**Refusals, each named before any work:**

| rule | when |
|---|---|
| `host.limit` | the request is over a limit |
| `host.malformed` | the request is not JSON, or is not the protocol's shape (unknown fields included) |
| `host.protocol` | the protocol id is wrong |
| `host.version` | the version is not 1 |
| `host.path-not-bytes` | bytes are owed and a path is given instead: a `file`/`uri`/`url`/`root`/`dir`/`location` key, or a member with no bytes that is not `withheld` |
| `host.member-path` | a member path is not basis-relative |
| `host.member-duplicate` | a member is declared twice |
| `host.member-digest` | a member's bytes do not match its digest |
| `host.basis-digest` | the basis digest does not match its members |
| `host.profile-name` | a profile name is not valid |
| `host.profile-in-basis` | a profile is also a basis member |
| `host.observation` | an observation is malformed or duplicated |
| `host.projection-kind` | a projection kind is not rendered by this build |

**Limits:**

| limit | maximum |
|---|---|
| input | 256 MiB |
| JSON values | 1,000,000 (depth 64) |
| members, profiles included | 50,000 |
| one member | 32 MiB |
| projections | 10,000 |
| response | 256 MiB |

This repository's own request is about 79 MB.

## Purity, and the two observations

A hosted run reads no repository and no file. It writes nothing, opens no
network connection and spawns no process. `79-host.sh` checks this with
strace.

Two readers on the model's path observe something outside the bytes:

1. **Signatures.** Whether an authorization or acceptance is signed is decided
   by `ssh-keygen -Y verify`.
2. **History.** Whether a gate run's sources moved since it ran is decided by
   `git diff --name-only <tree>` and `git ls-files --others`.

A hosted run runs neither. Each reader asks the request instead:

- A signature verdict is keyed by the sha256 of every byte the verdict depends
  on: allowed signers, principal, namespace, response, signature and key-list
  label.
- A git outcome is keyed by its arguments.

When the request supplies no observation, the reader fails closed, as a
missing `ssh-keygen` or `git` does. An unchecked signature is not a pass. The
response names each missing observation as an `unknown`
(`host.observation-missing`), and the run exits 2.

**The host is trusted for its observations** (owner decision, 2026-10-07).
A request that says a signature verified makes the hosted model treat it as
verified, and the same goes for a git read. The host that sends the request
is the trust anchor: it is expected to have run `ssh-keygen -Y verify` or
`git` itself, or to hold the verdict as an attested Node.
- `war host` does not check observations again. Its response keeps saying
  `authenticated: false`, which states that `war` itself checked none, and
  says how many were used.
- `war host --export` records the observations the standalone compiler
  actually made, so a standalone run and a hosted run of the same export
  agree.
- What the trust does not cover: a **missing** observation still fails
  closed, as above. A malformed or duplicated one is still refused
  (`host.observation`). The standalone compiler keeps verifying everything
  itself.

**Compatibility limits:**

- A repository with `[authority] store` keeps its store outside the
  repository, so outside any basis. A hosted run of it fails closed: the store
  authorizes nobody.
- Files the readers need must be members. Deliverable targets and gate inputs
  are read from the working tree, so `--export` includes the ones the build
  read.

## How Liminal runs it

1. Liminal holds the WAR records as Nodes and Relations, and their files as
   Workspace Basis members.
2. For a compile, it writes one request and runs the pinned `war host` with
   the request on stdin. The `war` binary is pinned by version, source and
   build.
3. It reads the response and stores the model, its diagnostics and the
   projections.
4. It keeps nothing of WAR semantics for itself. Where the model and Liminal's
   Nodes disagree, the response says so by rule. Liminal repairs its Nodes; it
   does not repair the model.

## Parity (SAS §82.3; LIM-SAS-RQ-015)

- `conformance/host/cases/` holds the request/response pairs. Each pair
  declares a byte observable (the whole response) and semantic observables.
  `run.sh` runs any host against them: set `HOST` to its entry point.
- `war --json model` and `war host --export | war host` give the same model
  bytes. The plant checks this on this repository (2,020 records) and on a
  scratch program. It also checks that every Warrant view the host renders
  equals the committed file.
- **Refusal controls.** A doctored fixture response fails its case. Flipping
  the supplied signature verdicts moves the hosted model off the standalone
  one, so the comparison sees observations and not only bytes.

## Projections, and the M6 seam

This build renders one projection kind, `warrant`: a Warrant's committed
views. OW-WAR-0148 M6 adds a pure render function (`war render`). Two
functions in `host.rs` are its seam, and both are empty until M6 merges:

- `m6_render_seam_kinds()` returns the kinds M6 renders.
- `m6_render_seam(corpus, model, kind, subject)` calls M6's render function.

Until then, a kind that is not listed is refused (`host.projection-kind`). A
listed kind with no renderer comes back `unavailable` with a reason.
