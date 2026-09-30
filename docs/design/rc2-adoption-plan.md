# RC.2 adoption and consolidation plan

Status: prepared for OW-WAR-0074; not an acceptance, authorization, supersession
or executed migration. Updated 2026-09-14.

## Exact decisions to carry forward

- The target is SAS 1.0.0 Stable. The owner designates the accepted historical
  edition RC.1; its record literally says 1.0.0 and retains its original bytes
  and signature. RC.2 is the current unaccepted candidate. SAS 1.1.0 is withdrawn.
- Canonical definitions belong in normative RC.2 §3.1. Examples and usage advice
  remain guidance in §3.2. Projection is a task-context selection; packet is its
  delivery object; a corpus index is a generated view.
- Preserve OW-WAR-0071/0072/0073 and their human authorization records. Prepare
  successor work when needed; do not amend their historical contracts in place
  or claim they were superseded by this planning act. The new adoption work
  handles the RC.2 basis; batch/UI workflow successors belong to Phase 3.
- Keep legacy CLI command meanings through Phase 2. New document/context
  commands are additive. No silent `war compile` alias changes.
- Preserve legacy resolution as an original fact. A new acceptance claim needs
  authenticated evidence for the same exact result and review meaning;
  qualification is separately evaluated against every baseline requirement.
  Legacy resolved state alone produces neither new record nor assurance mark.

## Adoption work

For paid autonomous execution, apply the owner's
[budget decision](autonomous-execution-policy.md): USD 10 per run by default,
configurable or explicitly unset, with reliable cost tracking required for paid
calls in every mode. This policy note is separate from source-set acceptance.

1. Review the final RC.2 SAS, format contract, schema, build scope and reference
   examples using their source-set manifest. The current digest is recorded in
   the roadmap and Warrant basis; no earlier digest silently covers this edit.
2. Prepare the owner-reviewed adoption ADR and record procedure. The old SAS
   command captures the configured source; running it today does not capture an
   arbitrary candidate path. The procedure must bind all normative companions
   and distinguish historical 1.0.0, RC.2 and future Stable identities. If tooling
   needs a change, amend this draft's scope and review it before implementation.
3. Present the exact source-set subject and migration implications for the
   existing human acceptance process. Commit/push is preservation, not acceptance.
4. Prepare explicit successor relations for any reused legacy work. Each must
   state which obligations, deliverables or evidence it adopts, if any. The
   selected route is not a supersession signature and retires nothing by itself.
5. Route new implementation context to RC.2 definitions plus the applicable
   format units and exact tests. Label imported legacy instructions by edition.
   Do not insert the whole legacy glossary as if it defined the new wire types.
6. Prepare edition banners for legacy guidance through the applicable change
   process. At the inspected base, CONTEXT.md and docs/SKILLS.md are authorized,
   unresolved OW-WAR-0068 deliveries; docs/DEFINITIONS.md is resolved under
   OW-WAR-0062/D-002. Inspect current pins before edits. Do not rewrite a resolved
   pin to make the banner cheap. AGENTS.md/template consistency and the separate
   README correction remain explicit integration checks, not hidden exceptions.

## Exact adoption subject: implementation proposal

Prepared under the owner's authorization of OW-WAR-0074 revision 1. Its response
signature and three attestation subjects were verified on 2026-09-14. This
authorizes preparation; it does not accept RC.2 or the following tool extension.

The current `Repository::sas_document` selects exactly one immediate Markdown
file under `paths.sas`. That directory also determines where revision history
is read. `sas::propose` hashes that document alone, and `accept_ingest` compares
that same configured document with the proposed digest. Merely naming a proposal
`1.0.0-rc.2`, or pointing configuration at the multi-file draft directory, cannot
establish acceptance of the candidate's complete source set.

The proposed extension has one responsibility: bind acceptance to an explicit,
immutable source-set subject while preserving existing single-document records.
The design and exact wire schema require review before implementation; names in
this table describe required meanings, not implemented API fields.

| Subject component | Required meaning |
| --- | --- |
| Edition | The edition being accepted, distinct from historical `1.0.0`, its retained RC.1 designation, and future Stable. |
| Main document | Exact source identity, byte length and digest, with the section 106 snapshot derived from those same bytes. |
| Manifest | Exact manifest bytes and digest; it identifies all review-package members and their normative/reference roles. |
| Members | All 23 currently listed members, each captured and checked against its declared path, length and digest. New candidates may have a different explicitly reviewed inventory. |
| Predecessor | An explicit retained acceptance identity; source selection must not hide or rewrite revision history. |
| Human act | Authorized signer, review meaning, adoption ADR and the complete subject identity covered by the signature. |

Capture source bytes before requesting acceptance. Present the captured subject;
acceptance must cover those immutable bytes, and changed input requires a fresh
proposal. Use the existing canonicalization and signing boundaries. Keep the
legacy digest domain and legacy records intact; any new domain/schema is an ADR
decision, not an implementation detail. Checking a manifest's own hash without
checking its members is insufficient.

The first implementation slice must pair a valid captured subject with each of
these refusals: changed main document, changed normative companion, changed
schema, changed reference fixture, changed manifest, missing member, ambiguous
or escaping member path, and inconsistent capture. Additional cases must refuse
reuse of historical acceptance as RC.2 acceptance and a candidate edition bound
to the configured legacy document. Signature refusal must leave no accepted
record. Legacy records must remain readable with their original identities.

Potential edit boundaries are `crates/openwarrant-core/src/sas.rs`,
`crates/openwarrant-cli/src/sas.rs`, source selection in `repo.rs`, signing
request/display handling, and focused fixtures. This is a proposed scope
extension: OW-WAR-0074's signed atoms remain unchanged. Before coding, prepare
the required reviewed amendment or separately authorized scope. The CLI SAS
module is pinned by resolved OW-WAR-0058; core SAS is pinned by OW-WAR-0058 and
OW-WAR-0062. Any change to those delivered paths requires the applicable
correction procedure; this proposal does not perform it.

The owner-allocated adoption ADR should record this subject contract, exact
candidate manifest digest, historical edition mapping, required correction
route, and activation rule. Implementation permission for downstream Warrants
begins only after their stated prerequisites and authorization hold. Batch
authorization and an in-app approval card remain separate workflow work.

## Cross-program requests retained from Knowledge Fabric

These requests are accounted for without claiming unimplemented facilities or
adding an undefined wire format to Phase 1.

| Request | Current fact and treatment | Follow-through |
| --- | --- | --- |
| Roadmap phase numbers above ten | The legacy RoadmapRef parser uses u8 and MAX_PHASE = 10, tied to OpenWarrant's old plan. This is a current implementation restriction, not a universal phase limit in the RC.2 document grammar. | F10 reports unsupported legacy references honestly. A generic roadmap parser change needs scoped fixtures, compatibility review and authorization before the legacy CLI can claim support. Do not treat a planning JSON edge as that fix. |
| Generated in-force requirement annex | A generated view can show requirements from an explicitly selected authoritative source revision; generation does not decide which revision has authority. RC.2 does not yet define a dedicated requirement-annex command or lifecycle record grammar. | Reuse capture/selection/provenance primitives; specify any new annex interface before adding it to build scope. |
| Retirement with no successor | Historical retention and current applicability are separate. The current RC.2 requirement table preserves and rescopes its own IDs; that is not an implemented generic requirement-retirement service. | A source owner must state and approve changed applicability and retain the old source. Do not invent a successor or a lifecycle record type merely to populate a relation. Any machine retirement contract remains an explicit extension to specify. |
| Append-only context cost | Retaining historical bytes does not require supplying every historical revision in every task packet. Caller capture, pointer selection and dependency closure determine the current declared basis. Required historical evidence still travels when referenced. | F03/F05/F07/F08 prove exact selection, closure, offline coverage and budgets. Record transfer size separately from entry-context size; no promised speedup. |
| Cross-repository sas:// resolution | RQ-005/RQ-060 preserve source and issuing authority. F3 v1 targets are explicit capture-root paths, not live URI fetches; holders/identities preserve origin. | F03/F10 may consume caller-prepared source bytes and explicit mappings. Online resolution and federation adapters remain Phase 3; ambiguous or missing required sources refuse completion. |

None of these requests permits accepting unknown metadata as an implemented
capability. In particular, the current Assumption type does not parse
`external_dependency`. New Warrant prerequisites use parsed blocking-assumption
fields plus a separately labeled planning index. Their ordering is a manual
authorization/performer hold, not automatic execution fencing.

## Evidence and routing-note corrections

The root checkout was f22ef2f when inspected. The handoff's cited correction and
performer fixes already precede that commit; there were no five extra commits
after it to merge. The temporary 2026-09-13 handoff path was absent. Current source
and authorization files were inspected rather than treating that note as proof.

OW-ADR-0019 and OW-ADR-0020 have proposed metadata. ADR-0019's prose requires the
unaccepted SAS 1.1.0; ADR-0020's prose requires OW-WAR-0073 authorization, which is
present. This status distinction is recorded, not silently normalized. All old
source and authorization bytes remain unchanged.

The separate planning checkout excludes the root's unrelated AGENTS.md and
pinned README edits. Its green structural check does not establish the root
checkout or the aggregate release gate as green. The root's earlier 305/1 battery
and 12/14 aggregate figures are handoff observations, not runs repeated here.

The ephemeral consolidated-base routing note can be archived after its facts
are retained here and in the source set. This plan and the source-set/roadmap
become the durable entry points; the original vocabulary proposal is discussion
history, not an additional normative authority.
