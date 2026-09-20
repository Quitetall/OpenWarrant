# Changelog

Notable changes to OpenWarrant. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/), with the caveat below.

## [Unreleased]

## [1.0.0-alpha.2] — 2026-09-19

### Security

- An authority record is believed because a human signed it, not because the
  file says so. Every authorization, resolution, correction and SAS acceptance
  is read through `authority_check`: a response for that act, over that digest,
  whose `.sig` `ssh-keygen -Y verify` accepts as the principal `roles.toml`
  binds to the actor, who must be `human` there. In 1.0.0-alpha.1 a hand-written
  `authorization.toml` naming anyone as authorizer passed `war check` with zero
  errors and satisfied §56.1 requirement 1.
- §56.1 requirement 1 ("the EXACT authorized Contract Revision") requires that
  signature, so a forged record cannot carry a Warrant to a resolution.
- `war dispatch` refuses a contract no human signed (`dispatch.unauthorized`);
  `--prototype` on `dispatch`, `run` and `perform` compiles anyway and records
  `prototype://unauthorized` as the authority acted under. `war console`,
  `war mcp` and `war perform --all` never pass it — §27.2 leaves an agent no way
  to decide it may work under no authority.
- Eleven planted violations exercise it, including a signature replayed onto
  another Warrant and a principal no key answers for. `docs/THREAT_MODEL.md`
  entry 11 states the residual: an agent can still WRITE a record; the refusal
  is on the read path.

### Added

- `war sign` offers a recorded-but-unsigned act as an ordinary pending row for
  all four act kinds. The act repeats the record — outcome, reason, kind and
  revision number all come from it — so a signature can only say what was
  already said. Reported as `*.signature-supplied` and journalled as
  `*.signature_recorded`, never as a second `*.recorded`.

### Fixed

- `war sign --all` no longer aborts on one act's failure, and an act needing a
  decision the batch cannot make (`--outcome`, `--kind`, `--adr`) is a
  `sign.needs-decision` warning naming the command rather than an error.
- One response filename per act kind. An authorization and a resolution of the
  same Warrant both bind `contract_digest`; sharing `<alias>.response.toml` made
  whichever signed second retire the first.


- Add candidate portable Dispatch transport and offline capability membership
  refusal through `war dispatch-bundle` and the SDK. Exact context bytes travel
  with their commitments; no execution, authority or semantic closure is claimed
  (OW-WAR-0023; `docs/cli/dispatch-bundle.md`).

- Add partial read-only `war preflight <alias>` with all six SAS readiness
  dimensions. Unobserved checks remain UNKNOWN and block readiness; no execution
  permission or recorded state changes (OW-WAR-0011, remaining scope documented
  in `docs/cli/preflight.md`).

- Add read-only `war inbox`: human-waiting Warrants, blocking hotline questions,
  known/unknown transition ages, and a proposed versioned JSON result. Existing
  signing policy, records and schema pack remain unchanged (OW-WAR-0070).

## [1.0.0] — released when the `v1.0.0` tag exists; until then this is the plan's ledger

**The protocol is stable from 1.0.0.** Every `oh.war/*/v1` record shape, every
`DigestDomain` string, the RFC 8785 canonical form and the schema pack
`openwarrant-schema-pack 0.2.0` are frozen: a change is additive with
`#[serde(default)]`, or it is a `v2` with a migration. Digests minted by 1.0.0
are reproducible by every later 1.x. `docs/COMPATIBILITY.md` states the rules;
`docs/roadmap/RELEASE_1_0.md` maps SAS §99's twenty-five criteria to what
proves each. Two acts precede the tag and are the owner's: the relicense to
Apache-2.0 (`RELICENSING.md`) and `war sign 1.0.0 --ssh-sign` accepting the
SAS revision; `release.yml` refuses to publish crates under any other licence.

Delivered under the 1.0 plan (each entry one slice, each slice one commit):

- The correction act (OW-WAR-0064, OW-ADR-0012): a resolved Warrant's pinned
  file moves for a reason through `war correct` + `war sign <alias>/<D-id>`,
  never silently and never by regenerating the record.
- `--json` on every command: one `oh.war/report/v1` envelope, errors included
  (§76.4). `war pins` (what a resolution pins) and `war next` (whose act is
  next; never a signing act for an agent).
- `AGENTS.md` is a template `war init` writes; `war agents-md` rewrites it.
- Draft Proposal v2 (OW-ADR-0013): operations carry payloads; `war plan
  --draft` runs a configured drafter, `--apply` creates the Warrant and records
  request, proposal, drafter run and pipeline under `plan/`.
- An async runtime for the MCP transport only (OW-ADR-0014); `war mcp` serves
  every read, request half and agent-permitted write over stdio with no
  signing or ingesting tool registered.
- A Claude Code plugin: the `openwarrant` skill with references, the MCP
  server, a `PreToolUse` pin guard and a `Stop` check on `war check`.
- Attestations (OW-ADR-0015): every ssh-signed act leaves an in-toto
  Statement in a DSSE envelope beside its record, signed with the same key
  under `oh.war/dsse`; `war attest` verifies signatures and subject digests,
  and `cargo xtask gate` runs `war attest --all`.
- `[project] performer` in `openwarrant.toml` (default `claude`), never a
  flag; `authorize` and `resolve` refuse an `effective_time` that is not RFC
  3339 UTC; `roles.toml.example` shows a second human signer, and two
  eligible signers is a question (`sign.who`), a duplicate principal a refusal.
- `war init --program <name>`: a SAS the tool reads (§6.10, §98, §106), the
  authority examples, the `war check` gate, and a first Warrant with real
  atoms; `war check` on the scaffold exits 0. `QUICKSTART.md` and
  `docs/EXAMPLES/01-code.md`.
- Rules in pinned files, landed as one correction batch: `roadmap.status-claim`
  (a hand-written **resolved** is refused), `roadmap.wrong-namespace` (a
  `roadmap://` ref names this program's §98), `sas.effective-time`; the
  corpus-status fallback namespace comes from `openwarrant.toml`; the Dispatch
  compiler keeps a wrapped bullet whole (dispatch digests move accordingly).
- `war watch`: the pending set once (`--once`, with `--json`) or as a live
  diff of what appears and what is signed away; `--notify-send` raises a
  desktop notification. A poller on the record trees, no new crate.
- `docs/sas/generated/NORMATIVE.{md,json}` (`oh.war/sas-normative/v1`): every
  SHALL / SHALL NOT / MUST / SHOULD / MAY sentence of the SAS with its section
  and the document digest, compiled by `war compile` and drift-checked by
  `war check --generated`; agents read it instead of the document.
- `CORPUS_TIMELINE.json` (`oh.war/corpus-timeline/v1`: every journal event,
  sorted, with a per-day histogram) and `CORPUS_PENDING.json`
  (`oh.war/corpus-pending/v1`: the human acts awaiting a signature, with the
  command for each), compiled, drift-checked, canonical, published by Pages;
  `war status --timeline` / `--pending`.
- The projection contract (`docs/PROJECTION_CONTRACT.md`): `CORPUS_STATUS.json`
  Warrants carry identity, contract revision and digest, obligations with
  dispositions and gates, deliverables with digest state and correction
  chains, gate runs with §44.6 classes, amendments, unknowns and record
  paths; top level carries `repository_url` and `generated_by`. Additive only
  after 1.0; a §84 Knowledge Fabric mapping is documented.
- The progress platform: `CORPUS_STATUS.html` is a single-file, no-framework
  app over the three inlined projections — dashboard, objectives, warrants
  (filters in the URL), warrant drill-down, milestone DAG, requirements,
  timeline, gaps, pending (with the command to copy), evidence; hash router,
  theme, keyboard rows; every count links to its rows, never a ratio. Every
  route renders against the committed data under a node stub DOM in the
  battery.
- `war export --progress <dir>`: the progress bundle (`oh.war/progress-bundle/v1`)
  — the corpus projections, the SAS normative projection and every compiled
  `WAR.json`, copied byte-for-byte with a sha256 manifest and a bundle digest;
  `--verify-progress <dir>` checks one.
- Stage-relevant context (SAS §47.2): a stage declares `context_atoms`,
  `context_sections` (`<atom>#<heading>`), `context_artifacts` and
  `context_external`; the Dispatch's §33 manifest includes the required
  atoms plus what is declared — a declared section narrows its atom as a
  selector — and omits the rest with a true reason. A missing section is
  refused with the headings that exist; `war dispatch --emit-context` writes
  the manifest beside the packet.
- Token accounting (SAS §33.7): every Dispatch carries `tokens` (a bytes/4
  estimate of its selected context, the budget it was compiled under, the
  method id) inside its digest; a stage's `budget_tokens` or `[context]
  default_budget_tokens` (32 000) is the budget, and over budget is
  `dispatch.over-budget` naming the three largest items; each compile is
  journalled as `dispatch.compiled` with its size.
- `war verify --bundle` writes the verification bundle
  (`oh.war/verification-bundle/v1`: request, atoms, deliverable bytes, plants,
  test names, gate runs, prior verifications, token estimate) and `--run`
  hands it to a configured `[verify] verifier_argv`, ingesting the answer
  through the unchanged seam; `docs/RESOLVING.md` describes the hand-off.
- The document work kind: `war document review [alias]` and the gate
  `document.review@1.0.0` — a Markdown deliverable at its digest, citations
  that exist (URLs recorded, not fetched), no placeholder in prose, and an
  `established` verification from someone other than the performer. The
  example is OW-WAR-0065, a research memo on the token approximation;
  `docs/EXAMPLES/02-document.md` walks it.
- The ops / runs work kind: `war run <alias> <stage>` runs a `service` stage's
  gate under the stage's `wall_time_seconds` (or `[run]
  default_wall_time_seconds`), mints a §44.6 receipt whose subject is the
  dispatch digest, and writes a Stage Submission requesting `verify` or
  `block` — never resolution; `war submit` ingests an external submission
  under the same refusals. Gates `ops.echo` and `ops.conformance.plants`;
  the example is OW-WAR-0066, `docs/EXAMPLES/03-run.md`.

## [0.1.0] — 2026-08-24

### Changed — BREAKING for agents that emit Draft Proposals

`DraftProposal` (`oh.war/draft-proposal/v1`) now carries
`#[serde(deny_unknown_fields)]`. A proposal containing any field outside §74.2's
surface is REFUSED at parse rather than silently ignored.

This is deliberate and is §91.8 tests 53 and 54. Previously an agent could send
`enterprise_id` or `authorized_by` and serde dropped them without a word, so an
agent that believed it had allocated an identifier had no way to learn otherwise.
Silently ignoring an attempt to exceed authority is worse than refusing it.

The protocol version is unchanged because the accepted field set is unchanged —
what changed is that fields outside it are now an error. An agent emitting only
§74.2 fields is unaffected.



### Beta opened — 2026-08-21

Alpha closed with all 40 Warrants resolved and OpenWarrant public. Beta is the
act of running it against real systems; nine Warrants (OW-WAR-0041–0049) are
authored against the SAS's own §98 phase-exit criteria, and none is discharged.

Recorded here because it changes how the alpha claim should be read: measured
2026-08-20, of twenty types implementing §40's epistemic classes, §46
independence, §56 resolution and §44.6 receipts, **twenty are referenced by no
code in `war` or the compiler.** They are implemented and unit-tested; no command
calls them. The capability exists — which is what alpha claimed — but `war check`
does not enforce these rules on a corpus, and a reader could reasonably have
inferred it did. OW-WAR-0046 wires them in and is the first beta task.

The README's "Not implemented" section was stale in both directions and is
replaced by three explicit categories: reachable from the binary, implemented but
unreachable, and not implemented at all.

### Added — Phase 1: the file-native compiler

- `war init` — initialize repository configuration and directories (SAS §71.1).
- `war new` — create a draft Warrant, with `O_EXCL` alias allocation so
  concurrent invocations cannot collide (§71.2).
- `war check` — deterministic, agent-free validation with PASS / WARN / UNKNOWN /
  ERROR diagnostics and a readiness verdict (§71.7, RQ-074).
- `war compile` — the full Markdown parent and RFC 8785 canonical JSON, each
  carrying the §17.1 generated header (§71.8).
- `war check --generated` — drift detection against committed projections
  (§17.3, RQ-075).
- Manifest and atom parsing with fail-closed validation for missing required
  atoms, duplicate ordinals, unknown required roles, fabricated enterprise
  identifiers, and composition cycles (§91.2).
- RFC 8785 canonicalization and the fifteen domain-separated digest domains of
  §65, pinned against the official conformance vectors.
- Parent contract-digest verification: a child citing a parent whose contract has
  since changed is reported as resting on a basis it was never authorized
  against (§20.2).
- First-class ADR atoms and a **generated ADR Overview** (§19.6, RQ-021): the
  Appendix A shape with a summary table, lifecycle buckets, and the complete
  decision bodies concatenated as one audit document. Drift-checked like any
  other projection, because §19.7 forbids a manually maintained index.
- `cargo xtask gate` — the aggregate gate of §92: SPDX headers, build, fmt,
  clippy, tests, licenses, and a planted-violation battery that asserts each
  control rejects for its intended reason.

### Decisions

- **OW-ADR-0001** — adopt `serde_jcs` for RFC 8785, chosen by running both
  realistic candidates against the official vectors rather than on reputation.
- **OW-ADR-0002** — parse atom frontmatter with a restricted reader instead of a
  YAML library, because the Rust YAML ecosystem offers only stale or pre-1.0
  options and a six-key header does not justify the attack surface of YAML 1.2.

### Known gaps

Stated rather than left to be discovered:

- Gate execution is not implemented. `war check` validates the record, not the
  work; a Warrant whose acceptance gates are nonsense passes.
- Preflight (§32.7) does not exist, so the verdict says WELL-FORMED rather than
  READY.
- Bound atoms (`ref =`) cannot be resolved offline; federation is Phase 4.
- Two of the nine projections in §17.5 are implemented (`full_warrant`,
  `canonical_json`).
- Contract revisions are not implemented; every compilation is revision 1.
- ADR supersession is not modelled at all — `AdrRecord` has no `supersedes`
  field — so §91.4 test 25 (supersession is acyclic) is not covered. The
  `superseded` status exists as a lifecycle state, but nothing records what
  superseded what.
- Conformance runs on one host, so §91.1 test 1's "two supported hosts" is
  satisfied by two runs rather than two architectures.
