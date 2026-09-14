# Context views and shared work

Normative companion to SAS RC.3 §19. These are standard/profile requirements,
not claims of an implemented database, compiler or workflow. View names are
provisional. Wire fields and provider transports must have fixtures before release.

## One source model, several views

The context provider SHALL assemble views from owned source records at exact
revisions. The master context is a generated assembly, not a second editable
authority. Shared identity, schema and revision references join project records
into one logical context model. Storage and indexes belong to the selected backend;
the OpenWarrant SDK has no mandatory SQL engine, database or model dependency.

| View | Content and meaning |
| --- | --- |
| Progress Context Packet | One supervisor entry with SAS and Warrant progress, pending work, document maturity, completion, qualification and pointers to exact specifications and detailed inventories. |
| Decisions and Domain Context Model | Retained ADR/decision revisions and experience records, with provenance, evidence and applicability. Observations and hypotheses retain their limits; they do not become binding decisions by appearing here. |
| Amendment and Scratchpad Tracker | Mutable draft atoms, proposed amendments and open questions. Writer ownership and revision checks prevent lost updates. Draft promotion preserves the cited revision and records the real act. |
| Contract Context Packet | Applicable API/schema/interface requirements plus code-derived facts and relevant connections, selected for tasks that use those contracts. Intended contract and observed implementation remain distinguishable. |
| Skill/context overview | Conditional pointers to relevant skills and agent instructions. Reuse the harness-supplied version when its exact identity is known; avoid duplicate instructions. |
| Out of Context Bucket | Selection or access state for excluded records, not a second source of truth. Suppression, quarantine and destruction have separate effects below. |

The supervisor's progress entry SHALL cover its authorized project scope. Large
inventories MAY use indexed or paged details reached from that entry. Coverage,
filters, source revisions and as-of identity are explicit. Unknown or withheld
state SHALL NOT appear as zero or complete. Document acceptance does not mean
implementation completion. Complete/unverified and complete/verified count as
completed work; qualification coverage is a separate measure (§13).

Workers receive the task's required packet, not every supervisor view by default.
Required rules remain exact with source revisions and full required dependencies.
Background may be summarized with provenance (§§7–9). Reference pointers alone do
not prove required content was delivered. The offline package remains self-contained
for required task context; it is not a substitute for the execution code workspace.

Code-derived contract facts SHALL identify repository, commit or captured-content
digest, symbol/schema, extraction tool/version and supported coverage. Differences
from an approved intended contract are reported as drift. Code extraction does not
approve the current implementation or establish undocumented behavior. A changed
source invalidates affected facts and projections through dependency links (§10).

## Task and query boundary

A task SHALL identify its Warrant revision, outcome/stage, participant repository
bases, actor/role, required outputs/checks and applicable permissions/limits. A
context request adds desired views, selection inputs and output budgets. The query
selects context for that task; it does not replace task identity or grant authority.

The LAMU integration SHALL support this query-driven projection model through a
typed request. LAMU may lower it to constrained, parameterized SQL. An optional
expert query surface requires a bounded read-only capability. SQL syntax and
physical database layout are backend concerns, not the portable Warrant format.
One logical shared store can use replicas without creating divergent contracts.

Providers SHALL admit sources under current access policy, capture a consistent
set of exact revisions, follow required dependency edges, select optional material,
validate coverage and emit a packet with its manifest. Release database transactions
before long-running agent work. A snapshot establishes captured consistency, not
semantic completeness or permission after capture. Recheck current permission and
revocation before disclosure and execution admission through the responsible adapter.

Vector search MAY rank optional background within the admitted source set. Required
rules SHALL be reached by identity and dependency edges, regardless of similarity
score. Shared storage or search access never grants access to another project's
restricted records. Knowledge Fabric or another record owner retains its authority;
indexing that owner's data in LAMU does not transfer ownership.

## Shared cross-project Warrant

Work requiring coordinated changes or deliveries from more than one project SHALL
use one canonical shared Warrant or adapter contract at an exact revision, including
work that preserves an existing interface. Local entries are references or
derived work orders. They SHALL NOT independently redefine common requirements.
This does not require every single-project task to become a shared Warrant.

The shared contract SHALL name participants, repository/source bases, common
interface obligations, per-project write scope, stage/dependency relationships,
required outputs/checks and actors or policies for affected-scope acts. Every
participant result retains its repository revision and the shared contract digest.
One team's permission does not grant write access to another team's repository.

Start conditions belong to the Warrant and its explicitly referenced governing
sources. Each condition identifies purpose (qualification-only or enforced action
gate), named action, timing, subject and required evidence. Conditions may express
independent starts, joint starts or integration after independent work. Explicit
all-of/any-of alternatives preserve governing requirements. Unknown required
predicates report unsupported/unknown; they never silently pass. A satisfied
declared alternative can pass an any-of group without satisfying a separate gate.

Unverified work has no universal approval gate. An explicit start/signoff gate
still blocks its named action. Qualification checks actual event timing; late
signing cannot satisfy a condition that required approval before work (§§5, 12).

Amend shared requirements once. Bind any required affected-scope approvals to the
same new revision and retain old attempts on their original basis. Participant
membership and approval requirements are explicit contract fields, not inferred
unanimity. Local stages may complete independently; the shared outcome completes
only when its declared integration conditions pass. Failure or absence on one
side remains visible without erasing completed local work. Merge order, compatibility
and rollback belong in stage constraints; database atomicity does not make Git
merges across repositories atomic.

Phase 2's first shared example SHALL cover OpenWarrant SDK document/record exchange
with LAMU, including both repository bases and interoperability cases. This SAS
does not allocate an official shared Warrant identifier or authorize sibling work.

## Exclusion, retention and assurance

Approved ADR/decision revisions and signed history SHALL retain immutable original
meaning. Successor or revocation records change current applicability without
rewriting the older act. Draft scratchpad edits remain mutable until a revision is
promoted or cited as retained evidence. Project-lifetime retention remains default.

| Operation | Required behavior |
| --- | --- |
| Context suppression | Keep source bytes/history; omit optional records from default projections. On-request inclusion follows current access policy. |
| Quarantine/revocation | Record reason, scope and authority; restrict access or applicability. Required inaccessible dependencies prevent affected complete packets. |
| Destruction under an authorized retention rule | Record exact evidence identity, actor/time, authority basis and availability. Preserve signed history and flag assurance that cannot be established from retained evidence. |

An active required rule SHALL remain in context while affected work continues.
Apply the work-change procedure in [work-stop contract](work-stop-contract.md):
stop affected work, preserve its checkpoint and old packet, apply the permitted
rule/archive transition, resolve the new basis, then resume. The user's explicit
continue-to-next-work-stop choice retains the old rule until that boundary. Archive
placement itself does not revoke a rule or authorize continued work without it.

Deleting evidence SHALL NOT delete linked authority records. Trace affected claims
through exact evidence dependencies, not a time bucket alone. Missing support does
not prove code failed, invalidate an authentic historical signature or imply review
never occurred. Report current ability to substantiate assurance separately from
historical acceptance. Deleting a redundant copy leaves support intact if exact
evidence remains available in an authorized retained copy. Restoring evidence allows
reassessment and preserves deletion history. Unavailability is not proof of deletion.
Existing disclosure to an agent cannot be undone; revocation controls subsequent
retrieval, cache reuse and work admission.

## Cost and conformance cases

Providers SHOULD use indexed exact lookups, content-addressed blobs, incremental
extraction and dependency-aware caches keyed by source, policy, role and profile.
A continuing receiver may use a delta only after acknowledging the exact retained
base; a fresh receiver needs a complete required packet. If required content exceeds
budget, narrow the task or obtain a larger permitted budget; do not silently omit it.

Measure query, closure, validation and rendering time, transferred bytes, initial
and total model tokens, stale-context failures and human effort separately. Model
ranking is optional and separately costed. No latency or token reduction is claimed
without observed comparative evidence.

| Case | Required observation | Phase |
| --- | --- | --- |
| CTX-01 | Supervisor view separates document maturity, work completion and qualification, with complete authorized coverage or explicit filters/unknowns. | 3 |
| CTX-02 | Provider resolves required rules by exact edges even when vector ranking misses them; tampered or incomplete packet refuses. | 2 |
| CTX-03 | Code-derived API facts retain source/tool identity; changed code invalidates affected view; observed drift cannot amend intended contract. | 2 |
| CTX-04 | Capture across projects has one explicit revision set; revoked permission blocks new disclosure; arbitrary SQL cannot broaden source access. | 2 |
| CTX-05 | Both sides reference one shared contract; local success with failed integration remains partial; amendments cannot drift between copies. | 2–3 |
| CTX-06 | Excluded optional content stays out of default projections; active required content stays until stop/change; quarantine blocks affected required output. | 2–3 |
| CTX-07 | Deleting sole evidence preserves signatures and flags affected support; retained exact duplicate and restored evidence are distinguished. | 1 supplied-record cases; 3 storage |
| CTX-08 | Delta with wrong or unacknowledged base refuses; fresh receiver gets complete context; budget controls preserve required content. | 2 |

Phase 1 represents these subjects and evaluates supplied record facts. Real query,
projection, extraction and storage behavior requires provider/workflow evidence.
