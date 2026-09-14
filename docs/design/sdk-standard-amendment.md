# SDK and standard amendment: review entry

Current product-direction draft: [SAS 1.0.0-rc.3](../sas/drafts/1.0.0-rc.3/README.md).
This records the owner's revised four-phase plan and skill-library adaptation
direction. It does not accept a SAS, alter a signed Warrant or change runtime policy.

The prior RC.2 compiler-focused roadmap remains retained history. Use the RC.3
source set and migration map for future planning. SDK parsing/authoring/record
helpers stay in OpenWarrant; LAMU and other providers own semantic compilation;
apps own workflow controls. Phase 3 proves a real webapp; Phase 4 qualifies Stable
and promotion. Prompt-only work needs no OpenWarrant approval or signature;
qualification-only requirements stay separate, while explicit Warrant start/signoff
gates are enforced. Release review can cover exact results in one signing ceremony.
The common Verified mark requires independent evidence and secure human acceptance.

Existing `docs/SKILLS.md` and `.claude/skills/war-*` already document adaptations
from Matt Pocock. The new skill contract makes their intended standard outputs,
attribution, context pointers and evaluation explicit. It records a migration from
legacy "a skill never signs" behavior; installed skill and CLI authority rules are
unchanged until scoped implementation and adoption.

Read the new SDK contract for boundaries, phase plan for acceptance gates, and
migration map for all existing implementation Warrants. The draft's validation
record identifies its checks and their limits. No production implementation,
integration qualification, signature, commit or push is implied by this document.

Consolidation complete: [context views and shared Warrants](../sas/drafts/1.0.0-rc.3/context-views-and-shared-work.md)
and [stop/change/recovery rules](../sas/drafts/1.0.0-rc.3/work-stop-contract.md) now
belong to the normative source set through SAS §§19–20. The source set also owns
prompt-only completion, explicit start gates and configurable work-stop pointers.
Concrete wire profiles, SDK operations and runtime enforcement remain phased
implementation tasks. Consolidating the draft does not sign or qualify it.
