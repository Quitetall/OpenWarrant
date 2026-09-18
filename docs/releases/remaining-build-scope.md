# Work remaining before Stable 1.0

The owner chose a separate workflow package inside OpenWarrant for the reference
webapp. The SDK remains independent of UI, service, database and agent harness.
This records that decision and prepares missing work; it does not change the SAS,
assign Warrant identifiers, approve contracts or claim phase completion.

## Existing Warrant frontier

| Warrant | Remaining work |
| --- | --- |
| OW85 | Final phase review; matching Linux/macOS observations now retained |
| OW86 | Complete Phase 1 prerequisites and freeze final real-provider qualification |
| OW87 | Implementation complete, unverified |
| OW88 | Implementation complete, unverified |
| OW89 | Packaging completed unverified; retain qualification/provenance limits |
| OW90 | Collect all four phase exits and exact release review subjects |
| OW91 | Owner-permitted publication, then download and verify exact public bytes |

Available evidence is frozen in [the review inventory](rc3-review-inventory.json).
A hash proves which bytes were listed. It does not establish independent approval.
OW90 cannot close while the Phase 3 product and its required observations are absent.

## Skill-driven local and cloud drafting

The owner requires both local agents and cloud models to draft Warrants using the
same `war spec` skill. The skill owns the method; OpenWarrant SDK owns parsing,
validation and authoring; the configured harness owns model routing and context.
Connected agents can use this path today through CLI/SDK without a model call inside
the SDK. Keep provider keys outside documents and packets. Preserve prompt, source
revisions, model/harness identity and validation findings in the drafting history.

The reference webapp still needs an agent-drafting action: send a bounded prompt
and exact context to a configured local/cloud adapter, validate returned proposals,
show a readable editable draft, then save through the existing SDK authoring seam.
Test local and cloud adapters with real observations, invalid output, timeout,
context limits and cost refusal. A deterministic fixture proves transport only.
Unknown cost must refuse when a hard spend cap applies; explicit uncapped policy
may permit it while displaying unknown cost. Drafting grants no execution or
qualification authority. Track this as a remaining workflow deliverable, not shipped UI.

## Prepare bounded implementation Warrants

These are proposed slices, not already-issued or completed Warrants. Each needs a
bounded contract and positive/refusal fixtures before dispatch.

1. **Reference package and local API (OW93 bounded draft slice complete, unverified).** Add a separate workflow package. Reuse SDK
   parsing, validation, authoring and immutable record semantics. Expose persistent
   local state and deterministic projections through an authenticated local API.
   Prove prompt-only drafting and unverified completion, preserved user docs,
   startup/restart, access refusal and invalid-input refusal. No agent launch yet.
2. **Web workflow and execution bridge (OW94 bounded configured-harness slice complete, unverified).** Display the generated project overview,
   filtered Warrant list, source/spec pointers and exact subject details. Start
   one eligible Warrant through an explicit connected-agent or harness interface.
   Preserve one worktree per Warrant, serialized writers, attempt identity and
   configured spend/retry limits. Prove changed-subject, impersonation, hard-cap
   unknown-cost and explicit verified-start refusal. A checkbox is an action
   request, not a signature or evidence of success.
   Extend the project entry to conform to the Master Document view in SAS RC.3 §19:
   short overview, vision/SAS/optional PRD summaries, exact source pointers and
   shared progress records. Provider owns assembly; app owns display. Existing
   progress viewer is not yet this complete Master view.
3. **Questions, stops and recovery.** Route in-scope questions to agents and
   governing questions to authorized humans. Pause affected work and dependents;
   preserve independent progress. Apply work and harness changes at the defined
   boundaries. Prove interrupted writers cannot resume after replacement; emit
   configured work-stop output only from committed tracker state. Exercise sync
   failure and retry without duplicate completion.
4. **First-party adapter contracts.** Freeze the inventory before phase freeze.
   Current SAS names LAMU, Knowledge Fabric, Katana, BLUT, Liminal and Bonsai;
   the owner confirmed all six. Use one shared contract for each cross-project
   scenario, participant-owned work and explicit stage requirements. Freeze real
   versions and receipt subjects. Stub results cannot qualify an unavailable app.
5. **Release review and real-user study.** Demonstrate one meaningful maintenance
   change, independent checks and secure human acceptance of an exact review
   manifest. Preserve individual result eligibility in a batch. Before any study,
   declare consent, sample size and acceptance thresholds in its Warrant. Measure
   setup and administrative time separately from substantive review and waiting.
   Record failures. Synthetic sessions and agents are not actual users.
6. **Hardening and release qualification.** Run native Linux/macOS installation,
   upgrade/rollback and recovery/fault cases. Exercise declared production and
   stronger profiles. Pin build provenance and dependency notices. Assign migration/version policy,
   stable schema/fixture freeze, compatibility and interoperability documentation,
   and maintainership procedures to this slice; review existing material and
   record missing work explicitly before phase exit. Supply results
   to OW90; only then can OW91 request publication permission.

Slices 1–3 can be developed independently of missing external adapters. Adapter
contracts can be prepared alongside them. Actual-user study depends on the working
webapp; final qualification depends on all required evidence. No global signature
ceremony is added to ordinary unverified execution.

## Decisions still needed before dependent work

- Owner confirmed LAMU, Knowledge Fabric, Katana, BLUT, Liminal and Bonsai.
  Freeze actual versions and shared contract scope before integration qualification.
- Owner approved preparing the three-developer study in OW-WAR-0095: ≤10 minutes
  active setup and ≤60 seconds administration for each participant. Obtain actual
  informed consent and freeze build/scenarios before running sessions.

Matching native OpenWarrant Linux/macOS observations now exist: [Phase 1](../warrants/OW-WAR-0085/implementation/native-20260918/comparison.json)
and [candidate installation](../warrants/OW-WAR-0089/implementation/native-20260918/comparison.json).
Their exact source subjects and limits remain binding. They do not establish
Phase 3 workflow/user evidence, final phase review or release qualification.

## Completion target

The requested target is 100% of the explicit Warrant inventory completed or
reconciled with evidence. Work completion, accepted reconciliation and qualification
remain separate facts. No denominator pruning, fabricated success or default
resolution can satisfy this target. Historical NOT SATISFIED outcomes retain their
meaning; map remaining useful scope to a successor or an explicit authorized
reconciliation. See [current reconciliation queue](warrant-reconciliation.md).

[Legacy gap review](legacy-gap-review.md) records concrete next actions; no historical disposition is changed.
