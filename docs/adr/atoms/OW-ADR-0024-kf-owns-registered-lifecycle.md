---
schema: oh.war/atom/v1
adr_uuid: 01a0e507-828e-761b-9320-a3b1a54f1e32
local_alias: OW-ADR-0024
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0d04c-5f41-7a13-9d41-14fd18657244"
---

# ADR OW-0024: Knowledge Fabric owns a registered Warrant's lifecycle; what stays local, and what each human act becomes

## Status

Proposed by the performer under OW-WAR-0126. **Not decided.** The three
blocking questions of that Warrant (`20-basis.md`, "Questions for the
owner") have no recorded answer: `war questions OW-WAR-0126` reports none
answered, and `rationale.toml` carries an empty `judgment_ref` for each.

- **Q-001**, how the tool knows a Warrant is registered;
- **Q-002**, what a local human act does on a registered Warrant;
- **Q-003**, whether a local authorization made before registration carries
  into Knowledge Fabric (a question for the owner **and** KF's owner).

OW-WAR-0126's work order says the performer drafts after those answers. This
draft was written before them, at the owner's request, so that the answers
have a concrete text to accept, amend or reject. Every section that depends
on an answer states the options, recommends one, and says it is not decided.
The act table below is written **under the recommended answers**; each row
says what changes under the options not recommended. When the owner answers,
the Decision section is rewritten to the answers, the options not taken stay
as the record of the choice, and status moves to `accepted` by the owner's
act, not the performer's. Until then this ADR governs nothing
(`AdrStatus::is_current` is false for `proposed`,
[adr.rs](../../../crates/openwarrant-core/src/adr.rs)).

## How claims are cited

- **SAS** sections are cited as `§n.m` of
  [the SAS](../../sas/WAR_Software_Architecture_Specification.md) at revision
  1.2.0 (sha256 `85f5e7ef…`, `docs/sas/revisions/1.2.0.toml`, state
  `proposed`; 1.1.1 is the accepted revision). The sections cited were
  read in the 1.2.0 text this repository carries; no comparison with 1.1.1
  is claimed.
- **OpenWarrant code** is cited by path in this repository at the commit
  this ADR is committed on.
- **Knowledge Fabric** is cited as `KF@3d3c871e:<path>`: the file at that
  path in the Knowledge Fabric repository at commit
  `3d3c871e44e9c62d36ed2d70eaeb46a9a86c9aed`, the commit OW-WAR-0126's basis
  names. KF is another repository, so these are code spans, not links; a
  reviewer checks them with `git show 3d3c871e:<path>` there. A statement
  about KF that has no such citation is in "Open questions for KF's owner"
  and nowhere else.

## Context

What the SAS already decides, and this ADR does not re-decide:

- §11 (table): Knowledge Fabric owns authorization, lifecycle, role
  authority, judgments and resolution. OpenWarrant owns WAR schemas, the
  canonical IR, validation, file-native authoring, the CLI, compilation and
  projections.
- §11.1: registering does not transfer source authority; Git may remain the
  Source Holder of authored atoms.
- §11.2: OpenWarrant SHALL NOT own a second institutional database. "When a
  WAR is registered, Knowledge Fabric owns authoritative lifecycle and
  controlled actions."
- §12.4: a WAR may remain valid as a local draft before allocation; it "may
  not claim globally authorized or effective state until registered through
  Knowledge Fabric". §12.6: offline creation keeps working.
- §66.2: before registration the local journal is draft history; after it,
  KF actions are authoritative and the journal "becomes a cache of action
  requests and receipts and SHALL not become a competing ledger".
- §67: the 32 controlled action names, the §67.1 envelope, §67.2
  server-assigned `recorded_at`, §67.3 optimistic concurrency, §67.4
  idempotency.
- §72.4: "The CLI SHALL not bypass KF authority."
- §27.2: an agent authorizes, resolves and annuls nothing.

What exists in this repository:

- Every human act is local. `war sign` performs it and writes a signed
  record in Git; the kinds are the `Pending` variants of
  [sign.rs](../../../crates/openwarrant-cli/src/sign.rs) (`act_name`:
  `authorize`, `resolve`, `accept`, `accept-roadmap`, `accept-standing`,
  `revoke-standing`, `correct`, `invalidate`), and `war sign --batch` signs
  many at once ([batch_cmd.rs](../../../crates/openwarrant-cli/src/batch_cmd.rs),
  OW-ADR-0019). `war check` believes the records through
  [authority_check.rs](../../../crates/openwarrant-cli/src/authority_check.rs).
- The §67 seam half exists: `war kf health` and `war kf act`
  ([kf.rs](../../../crates/openwarrant-cli/src/kf.rs), OW-WAR-0028) post one
  typed action, only under `--confirm-write`. The 32 names are
  [seam.rs](../../../crates/openwarrant-core/src/seam.rs)'s four vocabularies.
- Nothing in the tool knows a Warrant is registered. There is no
  `war register`. The manifest's `enterprise_id` must be empty: a non-empty
  value is refused as `ManifestError::FabricatedEnterpriseId`
  ([manifest.rs](../../../crates/openwarrant-core/src/manifest.rs)), which
  `war check` reports as `manifest.invalid`. Views render an empty one as
  `*not allocated*` ([render.rs](../../../crates/openwarrant-compiler/src/render.rs)).
- OW-WAR-0029 (registration, allocation, federation) and OW-WAR-0044 (the
  Phase 4 exit) are authorized and not delivered. OW-WAR-0029's
  `deliverables.toml` declares `identity.rs` and names no registration
  record file.

What Knowledge Fabric holds, cited:

- One `warrant` object per OpenWarrant UUIDv7; `create_warrant_draft` takes
  the UUID and refuses a non-v7 value or an existing identity; the phase is
  the lifecycle state along §24.7; the `warrants` group owns all 32 §67
  names "spelled as OpenWarrant's seam spells them"
  (`KF@3d3c871e:docs/decisions/0019-warrants-as-institutional-record.md`,
  "Decision").
- KF records the contract digest, Compilation Basis and canonical IR as
  OpenWarrant computed them and does not recompute; authored atoms never
  enter its database (same file, "Contract revisions are immutable
  snapshots").
- `submit_warrant` takes `contract_digest`, `compilation_basis` (each a
  64-hex sha256) and `canonical_ir`; `authorize_warrant_contract` refuses a
  `contract_digest` that is not the proposed one, and records the
  **authorizer as the actor posting the action** (`request.actorId`), with
  `authorization_meaning` and `policy_basis` from the payload
  (`KF@3d3c871e:packages/warrants/src/index.ts`, `submitWarrant`,
  `authorizeWarrantContract`). The payload has no field for a signature
  made elsewhere.
- `resolve_warrant` takes an `outcome` and sets it
  (`KF@3d3c871e:packages/warrants/src/index.ts`, `resolveWarrant`).

## What stays local, always

For every Warrant, registered or not. None of these is a human act, and none
is a lifecycle transition KF owns under §11:

| record or act | where it lives | after registration |
|---|---|---|
| drafting atoms, `war new`, `war plan` | the repository (Git is Source Holder, §11.1) | unchanged; §11.1 |
| `war check` | deterministic, offline (RQ-074) | unchanged; it additionally reads the registration signal (below) |
| `war compile`, projections, `generated/` | the repository (§11, OpenWarrant owns projections) | unchanged |
| the journal (`journal.jsonl`) | the repository | kept; per §66.2 it is a cache of action requests and receipts, never a competing ledger |
| gate runs and evidence receipts | local candidates (OW-ADR-0005) | unchanged; posting one is `attach_warrant_gate_run` / `register_warrant_evidence`, a separate act this ADR does not require |
| request halves: `war authorize`, `war resolve`, `war verify` (bundle), `war amend`, `war correct` | documents a human would sign; they write no record | unchanged as documents; their §67 counterparts are named in the act table |

## The act table

"Before registration" is today's behaviour, for every row: **this ADR changes
nothing an unregistered Warrant does** (§12.6, RQ-070, OW-WAR-0126 A-002).
The only change before registration is a label (next section), and a label
is a rendering, not an act.

"After registration" is under the recommended answers (Q-001 (a), Q-002 (a),
Q-003 (b)). *Refused (forwarded)* means the tool writes no record, exits
non-zero, and prints the §67 action request a human posts with `war kf act`
under their own KF role. *Refused (no KF path)* means no §67 action exists,
and the tool names that as a KF gap. Rule ids are proposed and **to build**
by the follow-on delivery Warrant; none exists in the tool today.

| act (`act_name`) | before registration | after registration | §67 action (`seam.rs` name) | refusal: rule id, and the input that triggers it |
|---|---|---|---|---|
| `authorize`, revision 1 | unchanged from today: a local, signed authorization, labelled local (§12.4) | refused (forwarded) | `submit_warrant`, then `authorize_warrant_contract` | `kf.registered-act-refused`: `war sign <alias>` (or `--ssh-sign`) with an authorization pending, `<alias>` registered |
| `authorize`, revision N+1 under an amendment | unchanged from today | refused (forwarded) | `propose_warrant_amendment`, then `authorize_warrant_amendment` (or `reject_warrant_amendment`) | `kf.registered-act-refused`: `war sign <alias>` with an amendment's revision pending, `<alias>` registered |
| `authorize` through a standing class (OW-ADR-0029, `war standing apply`) | unchanged from today | refused (forwarded) | `submit_warrant`, then `authorize_warrant_contract` | `kf.registered-act-refused`: `war standing apply <alias>`, `<alias>` registered |
| `resolve` (a new resolution, or the signature an unsigned recorded one lacks) | unchanged from today | refused (forwarded) | `resolve_warrant` (the agent's request half is `request_warrant_resolution`) | `kf.registered-act-refused`: `war sign <alias>` with a resolution pending, `<alias>` registered |
| verification ingest (`war verify --response`; not a `war sign` act, listed because OW-WAR-0126's work order names it) | unchanged from today | refused (forwarded) | `record_warrant_judgment` (proposed mapping; see open question KF-3) | `kf.registered-act-refused`: `war verify <alias> --response <file>`, `<alias>` registered |
| `correct` (OW-ADR-0012) | unchanged from today | refused (no KF path) | none: no §67 action exists | `kf.registered-no-kf-path`: `war sign <alias>/<D-id>` with a correction pending, `<alias>` registered |
| the batch act (`war sign --batch`, OW-ADR-0019) | unchanged from today | refused when any member is refused | none for the batch itself; each member maps as its own row | the member's own rule (`kf.registered-act-refused` or `kf.registered-no-kf-path`), reported through batch step 3, where one refusal refuses the batch and nothing is signed ([batch_cmd.rs](../../../crates/openwarrant-cli/src/batch_cmd.rs)): `war sign --batch --ssh-sign` whose pending set includes an act on a registered Warrant |
| `accept` (SAS acceptance) | unchanged from today | unchanged: it targets a SAS revision, not a Warrant | none | none; not changed by this ADR |
| `accept-roadmap` | unchanged from today | unchanged: it targets the roadmap record, not a Warrant | none | none; not changed by this ADR |
| `accept-standing`, `revoke-standing` | unchanged from today | unchanged: they target a class, not a Warrant; *applying* a class to a registered Warrant is the standing row above | none | none; not changed by this ADR |
| `invalidate` (a Gate Definition version) | unchanged from today | unchanged by this ADR (see open question OW-2) | none | none; not changed by this ADR |

Every name in the §67 column is one of `seam.rs`'s 32: `submit_warrant`,
`authorize_warrant_contract`, `propose_warrant_amendment`,
`authorize_warrant_amendment` and `reject_warrant_amendment` (contract group);
`record_warrant_judgment` and `request_warrant_resolution` (evidence group);
`resolve_warrant` (terminal group). No name is added.

**What each refusal does.** Both rules are errors. `kf.registered-act-refused`
writes nothing and prints the §67 request: the action name, the target
Warrant UUID, and the payload fields the action takes (for
`authorize_warrant_contract`: `contract_digest`, `authorization_meaning`,
`policy_basis`, per `KF@3d3c871e:packages/warrants/src/index.ts`). The tool
does not post it: posting is `war kf act ... --confirm-write`, run by the
human under their own KF actor and role headers, so KF records that human
as the authorizer. `kf.registered-no-kf-path` writes nothing and names the
missing §67 action as a gap for KF's owner.

**A registration signal that does not verify** (Q-001 (a)): `war check`
reports `kf.registration-invalid` on the Warrant, and every row above is
refused for it with that rule until the signal verifies or is removed by a
commit a human can see. Law 15: an unverifiable registration is neither
"registered" nor "not registered", and the tool does not guess in either
direction. Trigger: a registration record in the Warrant directory whose
receipt does not verify.

**Under the options not recommended.** Q-002 (b): the authorize, amendment,
standing and resolve rows become "signed locally, then forwarded" and the
refusal becomes the post's failure, reported after a local record already
exists; that is two records of one act, and §66.2 forbids the local one
being a competing ledger. Q-002 (c): every "after" cell reads "unchanged",
which §11.2 and §72.4 rule out; listed for completeness. Q-001 (c): the
refusal fires only when KF answers, and offline every act is UNKNOWN,
against §12.6.

## Labels before registration (§12.4)

Before registration a local authorization is real authority in this
repository and claims nothing institutional. Proposed labels, **to build**
in `war status`, `war show`, `WAR.md` and `docs/generated/CURRENT.md`:

- authorized, unregistered: `authorized (local; not registered with
  Knowledge Fabric)`;
- resolved, unregistered: `resolved (local; not registered with Knowledge
  Fabric)`;
- registered: the phase KF reports, prefixed `KF:`, beside the local record
  it replaced.

A view never writes "authorized" or "resolved" alone for an unregistered
Warrant. This extends the existing `*not allocated*` rendering of an empty
enterprise identifier ([render.rs](../../../crates/openwarrant-compiler/src/render.rs)),
which is today's only §12.4 label.

## Q-001 — how the tool knows a Warrant is registered (not decided)

### (a) A registration receipt file in the Warrant directory — recommended

KF's receipt for `create_warrant_draft` (and, once allocated, the enterprise
identifier) is written beside the Warrant and checked by `war check` the way
a signature is. It works offline (§12.6) and is visible in `git log`. Its
file name, schema and what it is verified against are **OW-WAR-0029's to
deliver**; this ADR refers to "OW-WAR-0029's registration record" and
invents neither. Whether KF's receipt can be verified at all is open
question KF-1.

Consequence: a forged receipt makes the tool refuse *more* (local acts on
that Warrant); deleting a genuine one makes it refuse *less*. The second is
the direction that matters, and it is a commit that removes a file, which
review of that commit can see. Residual risk R-003.

### (b) A non-empty `enterprise_id` in the manifest, only with such a receipt beside it

Rejected in the recommendation: it is (a) plus a manifest field, and it
requires relaxing today's `manifest.invalid` refusal of a locally set
`enterprise_id` (`FabricatedEnterpriseId`, §12.4, §91.3 test 20) to "refused
unless a receipt is beside it". Registration and allocation are also two
events: the identifier is allocated by a separate
`allocate_enterprise_identifier`
(`KF@3d3c871e:docs/decisions/0019-warrants-as-institutional-record.md`,
"What this does not decide"), so an `enterprise_id` marks allocation, not
registration.

### (c) Ask KF at each act

Rejected in the recommendation: every act needs the network, so offline
every act is UNKNOWN, against §12.6 and RQ-070.

## Q-002 — what a local human act does on a registered Warrant (not decided)

### (a) Refuse locally and print the §67 request — recommended

One authority per act. The human posts the request with `war kf act` and KF
records them as the actor. This is the table above.

### (b) Sign locally, then post the matching §67 action

Rejected in the recommendation: two records of one act, with a window in
which the local one exists and KF's does not (a failed post). §66.2 says the
local journal SHALL not become a competing ledger; a signed local record
that KF never received would be one. It also records the poster, not the
signer, as KF's authorizer unless the signer posts
(`KF@3d3c871e:packages/warrants/src/index.ts`, `authorizeWarrantContract`).

### (c) Keep signing locally and let KF mirror it

Contradicts §11.2 and §72.4. Listed only because the Warrant lists it.

## Q-003 — does a pre-registration local authorization carry over (not decided; the owner's and KF's owner's)

### (b) No: KF authorizes afresh; the local record stays as history — recommended

On registration the repository submits the contract (`submit_warrant` with
the contract digest OpenWarrant computed) and a human authorizes it in KF
(`authorize_warrant_contract`). The local `authorization.toml` stays, labelled
local, and its digest can be matched to KF's revision because both carry the
same contract digest. The KF authorizer may be the same person.

Reason: at `KF@3d3c871e:packages/warrants/src/index.ts`, KF's authorizer is
whoever posts the action and the payload has no field for a signature made
elsewhere. Carrying a local authorization over would record the poster as
authorizer, or need a KF change (open question KF-2).

### (a) Yes, submitted as the authorized revision with its signature as evidence

Not recommended while KF-2 is open. If KF's owner adds a field or evidence
path for a foreign signature, (a) avoids a second signature for the same
digest. Then the authorize row's registration path becomes "submit, then
post the local signature as evidence".

### (c) KF decides by organization policy

Possible under §11 (KF owns authorization). It moves the decision to KF
policy rather than settling it here, and the follow-on Warrant would then
need that policy's name to plant against.

The same question applies to a resolution recorded locally before
registration. Under (b), it stays local history and KF resolves afresh.

## Decision

*Pending Q-001, Q-002 and Q-003.* If the recommendations are taken: the
records and acts in "What stays local, always" stay local; the act table
holds as written, with the rules `kf.registered-act-refused`,
`kf.registered-no-kf-path` and `kf.registration-invalid`; views label local
authority as in "Labels before registration"; the registration signal is
OW-WAR-0029's registration record, checked like a signature; and a
pre-registration authorization does not carry over.

## Consequences (if the recommendations are taken)

- An unregistered Warrant behaves exactly as today, apart from labels.
- A follow-on delivery Warrant can plant each rule by its input: register a
  fixture Warrant (with OW-WAR-0029's record), run the act, and observe the
  rule id and that no record was written. It cannot start before OW-WAR-0029
  delivers the record, because the input that triggers every rule is that
  record.
- The correction act has no KF path. A registered Warrant's delivered file
  that drifts can be reported (`deliverable.digest-drift`) but not corrected
  until KF's owner adds an action (KF-4).
- No human act kind is added or removed. After registration, the acts the
  table forwards are exercised in KF for that Warrant, by a human under a KF
  role.

## Open questions for KF's owner

- **KF-1.** Are action receipts signed or otherwise verifiable by a party
  that is not KF's database? Q-001 (a) needs something to check.
- **KF-2.** Can `authorize_warrant_contract` (or `register_warrant_evidence`)
  carry a signature made outside KF, and record the signer rather than the
  poster? This decides whether Q-003 (a) is possible.
- **KF-3.** Is `record_warrant_judgment` the action for an obligation's
  verification verdict, or is `register_warrant_evidence` (§40.2) closer? The
  `work.warrant_judgment` columns were not compared against OpenWarrant's
  verification record.
- **KF-4.** Should §67 gain an action for correcting a resolved Warrant's
  delivered artifact (OW-ADR-0012), and one for a batch signature?

## Open questions for the owner, outside Q-001 to Q-003

- **OW-1.** The SAS text at revision 1.2.0 carries no §27.6 (the batch act);
  OW-ADR-0019 proposed it. This ADR cites OW-ADR-0019 and `batch_cmd.rs`
  for the batch act instead.
- **OW-2.** §11 lists the "institutional Gate Registry" as KF's. Whether
  `invalidate` moves to KF for gates a registered Warrant binds is not a
  Warrant lifecycle act and is left out of this ADR.

## Residual risks

- R-001 (from OW-WAR-0126): KF's action names or payloads change before the
  follow-on implementation. Every KF statement here names commit
  `3d3c871e`, so drift is visible by diff.
- R-002 (from OW-WAR-0126): this ADR is accepted before OW-WAR-0029 delivers,
  and its record differs from what Q-001 (a) assumes. The follow-on Warrant
  then supersedes this ADR rather than editing it.
- R-003: under Q-001 (a), deleting a genuine registration record re-enables
  local acts. Mitigation is review of the commit that deletes it; the tool
  alone cannot tell "never registered" from "record removed" without asking
  KF.

## Rejected alternatives

Listed under each question above: Q-001 (b) and (c), Q-002 (b) and (c),
Q-003 (a) and (c), each with its reason. They are the recommendation's
rejections, not the owner's, until the owner answers.
