# OpenWarrant RC.2 to Stable 1.0 implementation Warrants

Status: unsigned draft sequence, prepared 2026-09-14. Staging these Warrants
does not authorize implementation, accept RC.2 or publish a release.

The [current candidate](../sas/drafts/1.0.0-rc.2/README.md) and its
[build scope](../sas/drafts/1.0.0-rc.2/phase-1-build-scope.md) define the target.
The candidate source-set manifest SHA-256 is `201b707e9657a3c6674ea113a018a34a54b5ab425f37c417c8968af9f4d2cf31`.

The finish line is the standard plus the supported compiler/CLI. Stateful
workflows follow in Phase 3, starting with Knowledge Fabric Compiler; they
are not prerequisites for this Stable release.

## Sequence

| Warrant | Phase | Reviewable outcome | Required predecessor outputs |
| --- | --- | --- | --- |
| [OW-WAR-0074](../warrants/OW-WAR-0074/atoms/10-intent.md) | Preparation | Adopt the exact RC.2 build basis without rewriting history | None |
| [OW-WAR-0075](../warrants/OW-WAR-0075/atoms/10-intent.md) | Phase 1 | Parse and validate a minimal OpenWarrant document | OW-WAR-0074 |
| [OW-WAR-0076](../warrants/OW-WAR-0076/atoms/10-intent.md) | Phase 1 | Author valid documents from explicit fields | OW-WAR-0075 |
| [OW-WAR-0077](../warrants/OW-WAR-0077/atoms/10-intent.md) | Phase 1 | Capture immutable sources and resolve exact references | OW-WAR-0075 |
| [OW-WAR-0078](../warrants/OW-WAR-0078/atoms/10-intent.md) | Phase 1 | Evaluate context conditions with explicit uncertainty | OW-WAR-0075 |
| [OW-WAR-0079](../warrants/OW-WAR-0079/atoms/10-intent.md) | Phase 1 | Select complete context and expose conflicts | OW-WAR-0077, OW-WAR-0078 |
| [OW-WAR-0080](../warrants/OW-WAR-0080/atoms/10-intent.md) | Phase 1 | Build master context and faithful role projections | OW-WAR-0079 |
| [OW-WAR-0081](../warrants/OW-WAR-0081/atoms/10-intent.md) | Phase 1 | Export and verify portable offline context packages | OW-WAR-0080 |
| [OW-WAR-0082](../warrants/OW-WAR-0082/atoms/10-intent.md) | Phase 1 | Enforce context budgets and invalidate stale cache entries | OW-WAR-0077, OW-WAR-0080 |
| [OW-WAR-0083](../warrants/OW-WAR-0083/atoms/10-intent.md) | Phase 1 | Check supplied readiness and assurance records truthfully | OW-WAR-0075, OW-WAR-0077 |
| [OW-WAR-0084](../warrants/OW-WAR-0084/atoms/10-intent.md) | Phase 1 | Import legacy history without changing what was signed | OW-WAR-0077, OW-WAR-0083 |
| [OW-WAR-0085](../warrants/OW-WAR-0085/atoms/10-intent.md) | Phase 1 exit | Prove every Phase 1 primitive through a runnable driver | OW-WAR-0076, OW-WAR-0081, OW-WAR-0082, OW-WAR-0084 |
| [OW-WAR-0086](../warrants/OW-WAR-0086/atoms/10-intent.md) | Phase 2 | Expose the complete compiler through a machine-safe CLI | OW-WAR-0085 |
| [OW-WAR-0087](../warrants/OW-WAR-0087/atoms/10-intent.md) | Phase 2 | Expose noninteractive document authoring and validation | OW-WAR-0085 |
| [OW-WAR-0088](../warrants/OW-WAR-0088/atoms/10-intent.md) | Phase 2 | Make document authoring interactive and recoverable | OW-WAR-0087 |
| [OW-WAR-0089](../warrants/OW-WAR-0089/atoms/10-intent.md) | Phase 2 | Prepare installable compiler release artifacts and user docs | OW-WAR-0086, OW-WAR-0088 |
| [OW-WAR-0090](../warrants/OW-WAR-0090/atoms/10-intent.md) | Stable qualification | Qualify the integrated compiler and freeze the Stable source set | OW-WAR-0089 |
| [OW-WAR-0091](../warrants/OW-WAR-0091/atoms/10-intent.md) | Publication | Publish and verify OpenWarrant Stable 1.0 | OW-WAR-0090 |

The [adoption plan](rc2-adoption-plan.md) consolidates terminology, legacy
transition decisions and cross-program requests without widening the v1 wire
contract or claiming missing implementations.

## How to use the sequence

Start with the adoption package. Once its exact basis is accepted and the
parser Warrant is authorized, build F01 first, then F02. Authoring, capture and
condition work can follow their declared prerequisites. Record checks at each
public boundary; the Phase 1 exit independently reproduces the whole inventory.
Then integrate the CLI, prepare installable artifacts, qualify exact release
bytes and perform the final publication act.

Each draft contains intent, basis, work order, milestones, bounded obligations,
explicit blocking assumptions and a v2 proposal for review. No gate is declared
established and no human act is recorded. Dependencies are also available in
[the machine-readable planning index](rc2-implementation-roadmap.json).
That index is a planning aid, not a new standard schema. Current `war frontier`
orders internal stages; its OPEN status does not clear authority or external
prerequisites. `rationale.toml` makes those manual holds explicit in parsed fields. The current
parser ignores `external_dependency`; these drafts omit it. No automatic
cross-Warrant readiness mechanism is claimed.

Implementation uses one isolated worktree per Warrant, with one writer at a
time. The separate verifier reviews the exact approved result and evidence.
Internal steps may adapt within scope; format meaning, authority and acceptance
criteria changes return to the authorized decision-maker. The user may revise
this draft granularity before authorization.

## Existing work and release identity

OW-WAR-0067 covers the older release plan and must retain its actual history.
OW-WAR-0071/0072 concern the withdrawn SAS 1.1.0 and batch-signing plan;
OW-WAR-0073 concerns a native TUI. All three have human authorization records;
they are not unsigned drafts. The owner chose preserved history and successor work when needed; actual
supersession remains a separate governed act. This sequence does not silently supersede, resolve or
adopt those contracts. Only relevant authority, integrity and prerequisite
problems block new work; unrelated legacy closure is not an onboarding task.

Remote inspection on 2026-09-14 found `v1.0.0` already pointing at
`5ab3636597461e28712d777fed847150db6af8c3`, while GitHub listed only a
`v0.1.0` prerelease. The final publication Warrant therefore has an explicit
owner-decision blocker for the release ref/version strategy. Never silently
move the tag. All four crates.io sparse-index lookups returned HTTP 404 at
inspection time; recheck availability before publishing.

Stable completion requires observed public artifacts and fresh consumer checks,
not merely a tag, successful local test, or a queued release job. The agreed
chat completion signal is reserved for that verified publication outcome.

## Planning validation

See [the validation record](rc2-roadmap-validation.md) for observed draft checks,
source-set/example checks, remote evidence and their limits. Production RC.2
implementation, independent Warrant verification and Stable publication remain
future work.
