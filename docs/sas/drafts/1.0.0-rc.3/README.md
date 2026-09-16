# OpenWarrant SAS 1.0.0-rc.3

**Current product-direction candidate: standard and SDK, with external semantic
compilers and four delivery phases. Draft only; not signed adoption or Stable.**
Updated 2026-09-16.

## Review in this order

1. [SAS](WAR_Software_Architecture_Specification.md): normative ownership, optional
   verification, agent acts, secure human assurance and phase requirements.
2. [SDK contract](sdk-contract.md): standalone operations, caller/provider boundaries,
   proposed agent-act envelope and clearly bounded validation claims.
3. [Phase plan](phase-plan.md) and [Phase 1 scope](phase-1-build-scope.md): direct
   SDK proof then CLI, compiler integration, real webapp/users, hardening/adoption.
4. [War skill adaptation](skill-adaptation.md): Pocock methods adapted into Warrant,
   ADR, question, stage and review artifacts; attribution and context discipline.
5. [Migration map](migration-map.md) and [roadmap inventory](roadmap.json): every
   prior feature, case and planned Warrant assigned to its new owner/phase.
6. [Context views and shared work](context-views-and-shared-work.md): Master Document, evolving SAS, optional PRD, supervisor/worker views, shared contracts and retained evidence.
7. [Work-stop contract](work-stop-contract.md): completion, interruption, changes, live writers and recovery scenarios.
8. [Architecture amendment](sdk-ownership.adr.md), [decision map](decision-map.md)
   and [validation](validation.md): rationale, supersessions and observed limits.

The SDK handles documents and supplied records. LAMU or another provider resolves
cross-document context and constructs projections/packages. Workflow apps own
permissions, execution, signing transports and qualification issuance. OpenWarrant
may supply a small reference adapter; it does not build a second semantic compiler.

Humans and agents can execute ungated Warrants as prompt-only, unverified work
without OpenWarrant start approval, enrollment or signatures. A Warrant may
explicitly require a verified start or signoff gate; such gates remain binding.
Qualification-only conditions remain separate. Release review can obtain one secure human-signed
manifest covering multiple exact results. Each mark still requires its own baseline
conditions; late signing cannot invent pre-work approval. See the
[prototype and release examples](prototype-and-release-cases.md).

Unverified work reaches the same complete state as verified work. Every work stop
returns the configured safeword first and concise pointers to a deterministically
generated progress overview, implementation notes, document trail and next steps.
The overview holds progress indicators and metrics; configurable brevity saves
chat tokens. A new prompt starts the next work, subject to its explicit gates.

## Format versions and examples

The [format contract](format-contract.md) defines human-first RC.3 documents:
readable units first, compact TOML metadata in a framed footer. New unsigned SAS
sources use `oh.war/document/1.0.0-rc.3`. See [footer examples](examples-footer/README.md) and
[format decision](human-first-format.adr.md).
The [RC.2 examples](examples/README.md) and their packet/record wire identifiers
remain byte-identical compatibility inputs. RC.3 does not rename every schema or
digest domain. New agent-act meaning is in the SDK contract; no old acceptance
is reused for the new framing or assurance baseline.

The [source-set manifest](source-set.json) binds all normative companions and
reference files by exact bytes. It is a review inventory, not a signature. The
validation script checks source/metadata/link/inventory consistency and retained
example integrity. It does not implement or qualify an SDK/compiler/workflow.

## Next implementation boundary

First slice remains a minimal document parser and validator, now an SDK component.
Unsigned Warrant scopes name this exact candidate and permit prompt-only
unverified implementation. Formal adoption and human acceptance are required
for the associated qualification claims, not a universal prototype start gate. Existing RC.2
capture, human corrections and authorized work are retained in their own checkout;
this isolated drafting checkout does not claim those uncommitted records are here.
