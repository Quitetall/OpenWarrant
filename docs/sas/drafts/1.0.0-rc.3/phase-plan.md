# Four-phase delivery plan

Normative release plan for SAS RC.3. Every item is planned until observed. Phase
completion is distinct from Warrant resolution and from public release approval.

## Phase 1: Library and Standard

Deliver the standard, SDK scope/types/codecs/validators, agent-act and assurance
semantics, compatibility fixtures and adapted skill contracts. Implement each
SDK primitive through a direct runnable example or test driver, then expose the
same functionality through CLI commands. The [build scope](phase-1-build-scope.md)
names the inventory. Existing source-bound canonicalization may be reused.

Exit: every S01–S08 case admitted/refused as specified, no skipped required case,
exact source preservation, no false authority, working noninteractive CLI parity,
Linux/macOS evidence and required repository gates. Document and draft preparation
do not count as implemented APIs. No production semantic compiler or database
service is a Phase 1 deliverable.

## Phase 2: SDK and Compiler Integration

Deliver usable SDK interfaces, version/capability negotiation, typed adapters and
independent consumer examples. First-party LAMU implements semantic compilation;
the OpenWarrant team owns shared profile fixtures and SDK compatibility. Complete
RC.2 context-profile cases reassigned in [migration map](migration-map.md) using the
real provider and offline output validation. A second minimal consumer proves
that OpenWarrant is not coupled to LAMU-specific process globals or database state.
It need not duplicate an entire semantic compiler.

Exit: exact source/record/package exchange; required-rule preservation; unknown
applicability; cycles/conflicts; refused unauthorized inputs; budget failures;
tampering and semantic-omission controls; provider unavailable/malformed/mismatched
responses; version negotiation; repeatability and legacy preservation. Pin versions,
fixtures and result identities. Fake backend tests and schema validation alone do
not qualify real compiler integration. “Nothing fails” means all declared cases
pass with expected refusals, not a claim that no defect can exist.

## Phase 3: Workflow Integration

The [context companion](context-views-and-shared-work.md) assigns CTX-01–CTX-08
across Phases 1–3. Phase 2 must prove its query/packet/contract cases using the real
provider and one shared OpenWarrant/LAMU contract; Phase 3 proves authorized
supervisor views, retention effects, participant completion and integration state.
The [work-stop contract](work-stop-contract.md) supplies required Phase 3 scenario
families for work completion, interruptions, updates, live writers and recovery.

Deliver a real SDK-based reference webapp and test apps. The webapp is owned by
the workflow application layer; SDK core stays UI/service-independent. CLI remains
usable. Native TUI may be another consumer; it is no longer a prerequisite delaying
the first real webapp. Exercise the following first-party integration inventory:

| Application | Required reference scenario |
| --- | --- |
| LAMU | Compile source-bound context through the SDK profile and return inspectable outputs |
| Knowledge Fabric | Supply owned records/authorization facts and accept proposed updates through its controlled interface |
| Katana | Execute a bounded agent job, preserve attempt identity and return attributable evidence |
| BLUT | Run a bounded computational DAG step and return output/receipt identity without claiming human approval |
| Liminal | Supply one explicitly qualified source-semantic profile; preserve origin, conversion loss and unsupported cases |
| Bonsai | Supply code/source facts bound to the inspected revision; report stale or unavailable source facts |

These applications form the initial first-party inventory from the architecture
discussion. Confirm the full first-party inventory before freezing Phase 3;
additional first-party apps are added to the matrix with concrete scenarios.
“all apps” is not a hidden requirement to integrate every repository on the host.
Unavailable required adapters remain explicit blockers to the corresponding
Phase 3 claim; they do not block the standalone Phase 1 SDK.

Demonstrate an ungated Warrant through prompt → work/questions → completion
reported as unverified, without prior OpenWarrant authorization, enrollment or signing. Then demonstrate
release review → independent checks → one exact human-signed review manifest →
individual marks for eligible results. Missing OpenWarrant start approval must not
block the ungated path; missing qualification requirements must block the mark.
Separately demonstrate a Warrant's explicit verified-start/signoff gate blocking
its named action until satisfied, including refusal to bypass it in unverified mode.
Demonstrate refusal of resource-access violations, agent impersonation, failed
evidence, false pre-work approval timing and changed or uncovered batch members.
Both unverified and verified results reach the same complete work state. Return
the configured safeword on the first line and compact pointers to a deterministic
tracker overview, next steps, implementation notes and document trail at every
work stop. Test feature/integration/Warrant/program scope, configurable brevity,
record-derived progress indicators and metrics, and no automatic signal from an
ordinary safeword mention. Then start the next Warrant from a new prompt subject
to its explicit gates. Exercise tracker sync/projection
failure, concurrent changes and retry without duplicate completion or invented state.
Keep affected-work pausing, worktree writer serialization and bounded recovery.
Test the USD 10 default per-run cap, configured finite cap and explicitly undefined
cap. Paid calls with missing reliable accounting refuse under a mandatory hard cap;
unknown cost is never shown as zero. Test concise human approval/signing gestures
against exact subjects, including changed-subject refusal and stronger policy.
An explicitly selected professional workflow may require human start review, fixtures-before-work,
qualification before merge and separate deployment permission.

Exit: real webapp and real adapters exercised, a meaningful OpenWarrant maintenance
change completed, actual-user sessions recorded with consent and declared sample
size, friction and failures measured separately from substantive review. Agents
and synthetic fixtures alone are not actual-user evidence. Set study acceptance
thresholds in the implementation Warrant before running the sessions; do not invent
results or user counts in this plan.

## Phase 4: Hardening and Standard Adoption

Deliver security and reliability qualification, migration/version policy,
cross-platform packaging, recovery and fault-injection results, dependency/license
notices, interoperability documentation, stable schemas/fixtures and maintainership
procedures. Exercise prototype, documented production and declared stronger profiles.
Preserve actual source/actor identities and the human-mark boundary in every mode.

Exit: independent review and all required gates pass for the exact release artifacts;
install/upgrade/rollback and compatibility are observed; published claims name scope
and limits. The owner approves Stable publication and promotion. No phase or draft
automatically awards legal compliance, licenses or a security certification. A
specialized profile must define and substantiate its additional requirements.

Stable 1.0 and broad promotion no longer follow Phase 2 automatically. Provisional
SDK releases may occur earlier if clearly labeled. The owner's requested final
chat word `pineapple` belongs only after actual stable publication, not this plan.
