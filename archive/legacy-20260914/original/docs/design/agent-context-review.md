# Agent-readable artifacts and compiler projections

> **2026-09-14 update:** The owner directed completion of SAS **1.0.0-rc.2**
> and testable Phase 1 scope before building. The [RC.2 source set](../sas/drafts/1.0.0-rc.2/README.md)
> now consolidates these decisions, chooses the candidate format and assurance
> contracts, and provides examples and a build matrix. This document retains the
> earlier interview/research history; statements below that a mechanism remained
> open describe that earlier stage. Use RC.2 for the current candidate design.
> RC.2 is not yet accepted or implemented.

Status: discussion draft, 2026-09-13. Source inspection at
`f22ef2f7282e8b5c72f2c4323b300f3b5e16102c`, with existing local design edits.
This review proposes changes; it does not amend the accepted SAS, authorize
implementation, or establish improved agent performance.
Following C13, the reviewed requirements are integrated into the
[working SAS 1.0.0](../sas/drafts/1.0.0-rc.2/WAR_Software_Architecture_Specification.md).
C14 designates the earlier baseline 1.0.0-rc.1. The working draft remains unaccepted.

## Decisions and method

- C12: the official OpenWarrant standard and compiler reach v1.0 at completion
  of the CLI/Compiler phase. Workflow integrations follow.
- Q51: v1 pointers support structured conditions plus explicit references.
  Condition syntax remains open.
- Q52: when an available rule's pointer cannot be evaluated because an input is
  missing, include the rule and its dependencies and flag uncertain applicability.
  Permission and architecture blockers remain separate.
- Q53: v1 exports a portable offline package with the task brief, exact rules,
  required dependencies, and source manifest. A resolver is optional; consuming
  required context does not depend on the original repository or a running service.
- Earlier Q37-Q39: assemble context from sources, preserve applicable binding
  rules exactly, and compile complete valid inputs without a model call.
- Use the installed Matt Pocock `writing-great-skills` and `writing-for-agents`
  references: conditional pointers, progressive disclosure, co-location,
  checkable completion criteria, one source per meaning, and measured pruning.

These writing principles suggest experiments. They do not prove that fewer
tokens improve agent outcomes. Binding clauses and safety requirements cannot
be deleted because a model is expected to know them. Stable terminology can
save repetition, but a short word cannot replace an exact constraint.

## What current sources establish

1. **Useful context machinery already exists.**
   [SAS §§33 and 47](../sas/WAR_Software_Architecture_Specification.md) require
   exact revisions, source precedence, provenance, omission records, and budget
   handling. [Context types](../../crates/openwarrant-core/src/context.rs) model
   required items, conflicts, trust, and holders. The
   [Dispatch compiler](../../crates/openwarrant-compiler/src/dispatch.rs) refuses
   omitted required atoms and produces deterministic bytes for fixed inputs.

2. **Keyword extraction can remove necessary meaning.**
   The SAS has 19,874 whitespace-separated words; its generated
   [NORMATIVE view](../sas/generated/NORMATIVE.md) has 7,181. These are word
   counts, not token measurements. The
   [extractor](../../crates/openwarrant-core/src/normative.rs) selects sentences
   with uppercase normative keywords and skips code fences and table rows.
   In §33.8, it omits the sentence defining inherited trust, classification,
   and taint, retaining only the prohibition against laundering influence.
   It is an index of selected sentences, not proof of sufficient task context.

3. **Required selection is still coarse.**
   The [selector](../../crates/openwarrant-cli/src/context_select.rs) includes
   every required atom. A section selector on an already included atom adds
   heading metadata; its digest and byte accounting still cover the whole
   atom. Optional atoms can be selected by section. The compiler derives
   required sources from required atoms, not from a graph of rule dependencies.
   Q38 requires exact applicable rules; it does not require every stage to
   carry every paragraph of every required source file.

4. **Pointers need identities and dependency semantics.**
   Current section selectors use heading text. The
   [section reader](../../crates/openwarrant-core/src/sections.rs) returns the
   first matching heading; renaming a heading changes the selector. Current
   context items record source and section metadata but no conditional trigger
   or explicit rule-dependency edges. Q51 adds that design requirement.

5. **A manifest reference does not establish content delivery.**
   The [Dispatch command](../../crates/openwarrant-cli/src/dispatch.rs) emits
   a context manifest only when requested. The
   [performer path](../../crates/openwarrant-cli/src/perform.rs) requests no
   manifest output and sends serialized Dispatch JSON to the child. That
   JSON refers to the context manifest by artifact URI and digest. The child
   has the repository working directory, but this handoff does not supply a
   portable resolver or the selected source bodies. Q53 now requires portable
   offline delivery; the package interface still needs definition.

6. **Glossary and instructions duplicate context.**
   [CONTEXT.md](../../CONTEXT.md) contains 761 words and is selected in full
   whenever present. It defines current strict lifecycle rules as well as
   terms. [AGENTS.md](../../AGENTS.md), the
   [OpenWarrant skill](../../.claude/skills/openwarrant/SKILL.md), and its
   references repeat rules and the full lifecycle. Some small shared guardrails
   may earn their repeated placement; the entire lifecycle need not be loaded
   for every operation. Existing definitions also need revision for the newly
   selected distinction between valid drafts, execution, and assurance.

7. **Workflow requirements appear in general authoring templates.**
   The [program SAS template](../../crates/openwarrant-cli/templates/PROGRAM_SAS.md.tmpl)
   starts with an adoption workflow, human signature requirements, and fixed
   numbered sections consumed by the tool. The
   [adoption work order](../../crates/openwarrant-cli/templates/adopt/40-work-order.md)
   describes the lifecycle as deliverables. Those templates are useful for
   that workflow, but cannot define the minimum document format selected in
   Q41 and Q48. Profile-specific rules need a visible boundary.

8. **Verifier evidence already has a separate surface.**
   The [verification bundle](../../crates/openwarrant-cli/src/bundle.rs) includes
   obligations, atoms, deliverable excerpts, gate runs, and prior verifications.
   Excerpts expose truncation and a full-file digest. Preserve this distinction
   from implementation guidance. Add a defined retrieval path for full evidence
   when an excerpt cannot settle an obligation; truncation is not full review.

9. **Budget accounting needs a defined boundary.**
   Current [token accounting](../../crates/openwarrant-core/src/tokens.rs) uses
   bytes divided by four, rounded up. The Dispatch path applies it to selected
   source byte counts. It is labeled as an estimate. It does not measure a
   model's final input after JSON wrappers, tool definitions, runtime history,
   and later retrievals. Compiler size and harness context-window accounting
   need separate, explicit responsibilities.

These are source-backed observations over representative artifacts and the
relevant compiler paths. No new runtime, model evaluation, or full corpus audit
was performed for this review.

## Proposed artifact improvements

| Artifact | Proposed improvement | Completion evidence |
| --- | --- | --- |
| SAS | Give each requirement a stable ID; keep its conditions, definitions, exceptions, and dependency references together. Separate core format, compiler conformance, assurance profiles, and future workflows. | A selected requirement brings every declared semantic dependency with it; references survive heading changes. |
| Warrant | Lead with outcome, bounds, required outputs, applicable constraints, and how each outcome is checked. Keep implementation advice distinguishable from binding outcomes. | An agent can identify permitted work and the evidence needed to return it without reading unrelated lifecycle instructions. |
| ADR | Keep decision, applicability, authority/status, and rule dependencies together. Put supporting discussion behind a precise pointer. | Accepted and proposed decisions cannot become interchangeable through projection. |
| Stage | Carry the current objective, inputs, outputs, dependencies, and an observable stopping condition. Keep later execution instructions in later stage packets. | Each stage has a checkable result; an implementation packet does not instruct its agent to perform human acceptance. |
| Context pointer | Name the exact target, its role, the condition for including it, and dependencies needed to interpret it. Generate a readable explanation from the same metadata. | Condition evaluation and target resolution are reproducible, with visible missing inputs and reasons for selection. |
| Glossary | Define each term once. Include terms required by selected rules and instructions, plus their definition dependencies. Keep a full glossary available by pointer. | Removing an unrelated definition changes size without removing meaning needed by the task. |
| Evidence and questions | Present the relevant observation or unresolved question first; link exact raw evidence and decisions. Distinguish observation, agent claim, and authorized decision. | Every conclusion can be traced to its evidence; a question's answer affects only its declared dependents. |
| Skills and agent instructions | Keep a small entry point with operation-specific routes. Generate or reference shared rules from their actual authority source. Use exact completion criteria. | Agent trials reach the needed reference without repeated human routing or premature completion. |

The generator should help authors supply metadata and links. The compiler should
derive indexes, manifests, and views. Authors should not maintain duplicate
pointer catalogs, copy hashes by hand, or repeat the same rule in each Warrant.
Automatic link proposals still need a reviewable account of what was selected.

## Proposed projection delivery

Compile source documents into a graph of addressable content and declared
dependencies, then derive a packet for one role and work stage:

1. **Task brief:** outcome, scope, permissions supplied by the governing workflow,
   required outputs, open blockers, and stopping conditions.
2. **Binding context:** exact applicable rules with the definitions, conditions,
   and exceptions needed to interpret them. Resolve declared dependencies before
   describing the packet as complete.
3. **Reference catalog:** conditional pointers to supporting explanations,
   examples, repository sources, and evidence. Keep background summaries marked
   as derived and tied to their sources.
4. **Manifest:** exact source revisions and digests, selection reasons, omitted
   items, unresolved inputs, conflicts, representation, and size-accounting method.

The machine representation and readable view should derive from one intermediate
representation. Q53 requires an offline package containing all required context
bytes and its source manifest; a connected caller may additionally use an optional
resolver over the same immutable content. Clearly distinguish optional references
from content actually supplied. Required context must remain readable without
repository or service access. Access restrictions apply to resolved content as
well as the initial packet. This context package does not supply an execution
workspace. Its layout and optional-background policy remain open.

Example pointer meaning, not proposed wire syntax:

> **Session changes:** when the declared scope includes session handling, include
> the exact session-expiry rule and its definition dependencies from the pinned
> architecture revision. Offer the design rationale as supporting reference.

Under Q52, missing scope metadata in this example causes inclusion of the rule
and its dependencies with an uncertainty notice. Keep the selection condition
unknown and retain the rule's actual conditions. Including text does not grant
permission, resolve an architecture question, or remove access and budget limits.

The compiler evaluates declared inputs. It cannot establish that those inputs
describe every file the agent will eventually change. If work scope expands,
the workflow must re-evaluate affected pointers and refresh or revise context
under its approval policy. Record the changed basis; do not silently rewrite
an earlier packet.

Required rules should not disappear to meet a budget. Proposed remedies are
removing unrelated background, supplying a smaller authorized stage, or using a
larger permitted budget. Keeping one source per meaning does not forbid repeating
an exact rule across separately compiled packets that each need it.

## V1 proof candidates

These are proposed acceptance scenarios, not completed tests or a final release
gate. The test corpus should cover SAS, Warrant, ADR, and evidence inputs.

- Compile the same captured inputs without a model and obtain identical output.
- Select a rule whose exception, definition, and scope live elsewhere; include
  all declared dependencies. Preserve the §33.8 inheritance sentence in a
  summary-related packet.
- Exercise matching, nonmatching, and missing condition inputs. For Q52's missing
  input case, include the available rule and its dependencies, record uncertain
  applicability, and preserve its conditions. Confirm that permission and
  architecture blockers remain effective. Expose each selection reason; reject
  malformed conditions and unresolved required targets.
- Reject ambiguous or stale targets; handle renamed headings through stable IDs.
- Test dependency cycles and conflicting sources with explicit diagnostics.
- Preserve binding bytes when shortening background; retain source provenance,
  classification, and trust labels through summaries.
- Load every required item from a fresh offline consumer using only the exported
  package, with the original repository inaccessible and no service running.
  Test missing content and digest mismatch. Mark absent optional content accurately.
  If a resolver is supplied, test it against the same immutable content; its
  absence must not prevent reading required context.
- Exceed the packet budget with required content and observe a named refusal.
  Report estimates as estimates; account for retrieved context at the caller.
- Build preparation, implementation, verification, and human-review views from
  the same governing result. Preserve common constraints without passing an
  implementer's suggested verdict off as independent verification.
- Compare full-source, current extracted-sentence, and proposed task packets on
  the same agent tasks. Measure required-rule omissions, wrong actions, successful
  outcomes, retrieval failures, total tokens, elapsed time, and human assistance.
  Use repeated trials and include tasks where a shorter packet should fail.

## SAS consolidation map

At inspection, the registry reported accepted revision 1.0.0 matching the source.
The owner subsequently clarified that this earlier baseline is designated
1.0.0-rc.1 and the same SAS is being refined for 1.0.0 Stable. The
[SAS index](../sas/drafts/README.md) preserves the distinction between the corrected
release designation and the original signed record identity. The older
[proposed 1.1 batch/rendering ADR](../adr/atoms/OW-ADR-0019-sas-1-1-0-batch-act-and-renderings.md)
is not the target version for this work.

The context and packet requirements now live in the
[working SAS](../sas/drafts/1.0.0-rc.2/WAR_Software_Architecture_Specification.md).
The [integration checklist](../sas/drafts/1.0.0-rc.2/README.md) names its coverage and
remaining decisions. Wider consolidation continues from the
[product draft](openwarrant-product-spec.md) and [decision ledger](openwarrant-friction.md).

Use the full ledger as the consolidation checklist. For each Q1-Q53 and C10-C14,
record the target requirement, superseding decision if any, applicable profile
or phase, and planned acceptance evidence. Do not promote every earlier answer
unchanged: later answers explicitly narrow several of them.

| Decision family | Existing SAS areas to revise or relocate |
| --- | --- |
| Standard ownership, minimal Markdown sources, validity versus readiness (Q36-Q42, Q48) | §§2, 6-11, 16-17, 28, 32, 61-63, 79-81. Preserve source history; define the new authoring contract. |
| Master context, exact rules, no-model compilation, conditional pointers, offline delivery (Q37-Q39, Q51-Q53) | §§33, 47, 61-65, 68, 81, 88-89, 94. Add dependency, portable package, optional resolver, invalidation, conservative inclusion for unknown applicability, and conformance rules. |
| Outcomes, advice versus constraints, fixtures, independent verification, assurance mark (Q1, Q3-Q6, Q20, Q43-Q47, C10) | §§22-23, 28, 38-47, 51, 56. Separate document validity, execution progress, and human-backed qualification. |
| Permission policy, automatic operation, human acts, acceptance, merge and deployment (Q8-Q11, Q15-Q16, Q49) | §§25-28, 38, 53-56, 76, 85, 101. Reconcile earlier delegation choices with later human-backed mark rules. |
| Progress, hotline, recovery, rebasing, repairs, limits, harness protections (Q7, Q18-Q19, Q21-Q27, Q33-Q34) | §§23-24, 32, 44, 47-55, 75-76. Preserve as workflow/harness contracts where applicable, without making every workflow part of compiler v1. |
| Successor work, retention, setup, migration, friction, real-work proof (Q2, Q13, Q28-Q31, Q35) | §§6-7, 29-31, 34, 36-38, 52, 56, 68, 89, 94, 97-99. Preserve historical evidence without freezing ordinary successor development. |
| Local clients, platforms, optional adapters, phases and release boundary (Q12, Q14, Q17, Q32, Q50, C11-C12) | §§2, 9-11, 48-50, 75-81, 93, 98, 106. Replace old phase ordering and mandatory sibling-runtime assumptions. |

In particular, §11.3 currently mandates Liminal for the final production
compiler. That conflicts with the selected standalone standard/compiler v1.0.
The older proposed batch/rendering ADR also predates authenticated-session
approval and autonomous prototyping decisions. Resolve these conflicts explicitly
in the successor draft and governing ADRs; prose pruning cannot settle them.

Open discussion: condition vocabulary; authority over
scope metadata; dependency declaration and completeness limits; offline package
and resolver interfaces; exact assurance baseline; migration/versioning; final
v1 conformance gates. None is silently settled by this review.
