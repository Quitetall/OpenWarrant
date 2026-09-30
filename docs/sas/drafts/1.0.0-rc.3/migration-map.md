# RC.2 to SDK roadmap migration

Reference companion to SAS RC.3. Unsigned contracts OW-WAR-0075–0091 now carry these SDK-era assignments. No
signed subject, delivery pin, journal state or authorization was rewritten.
Original proposal.json files remain historical drafting inputs; atoms are current.
Original case descriptions remain in the [RC.2 scope](../1.0.0-rc.2/phase-1-build-scope.md).
Machine inventory: [roadmap.json](roadmap.json).

## Feature reassignment

| RC.2 feature | New responsibility | Primary phase | Owner |
| --- | --- | --- | --- |
| F01 | S01 | 1 | SDK |
| F02 | S01 | 1 | SDK |
| F03 | source types S03; capture/resolution integration | 2 | provider + SDK types |
| F04 | condition syntax S03; evaluation integration | 2 | provider + SDK syntax |
| F05 | selection/closure integration | 2 | provider |
| F06 | assembly/projections integration | 2 | provider |
| F07 | integrity S05; package construction and semantic validation integration | 2 | provider + SDK integrity |
| F08 | budget/cache integration | 2 | provider |
| F09 | S04 | 1 | SDK; runtime enforcement Phase 3 |
| F10 | S05 | 1 | SDK |
| F11 | S02 | 1 | SDK |

T01–T10 and T42–T56 stay SDK Phase 1. T11–T41 move to external compiler
integration in Phase 2. T33/T35 include additional SDK integrity subsets. F04
syntax remains in SDK S03; runtime applicability and closure stay provider-owned.
Every mutation in a case remains required. S06–S08 and SDK-01–SDK-24 add CLI,
consumer, agent-act, skill, prompt-only, action-gate, work-stop response and release-review cases; they do not
replace any old context control. Legacy permission/refusal cases retain their
original profile scope; they cannot imply a universal block on unverified work.

## Warrant planning assignments

| Existing Warrant | Proposed action | Owner / phase |
| --- | --- | --- |
| OW-WAR-0074 | preserve signed contract; separate successor adoption | OpenWarrant adoption / adoption |
| OW-WAR-0075 | revise unsigned basis; retain parser scope | OpenWarrant SDK / 1 |
| OW-WAR-0076 | revise unsigned basis; retain field authoring | OpenWarrant SDK / 1 |
| OW-WAR-0077 | move capture/resolution implementation; keep SDK source types | LAMU/provider; SDK boundary / 2 |
| OW-WAR-0078 | move condition evaluation; retain SDK syntax validation | LAMU/provider; SDK boundary / 2 |
| OW-WAR-0079 | move selection/dependency closure implementation | LAMU/provider / 2 |
| OW-WAR-0080 | move master/projection implementation | LAMU/provider / 2 |
| OW-WAR-0081 | move package production/semantic checking; retain SDK integrity inspection | LAMU/provider; SDK boundary / 2 |
| OW-WAR-0082 | move context budget/cache implementation | LAMU/provider / 2 |
| OW-WAR-0083 | revise record/agent-act/assurance scope; runtime acts stay Phase 3 | OpenWarrant SDK / 1 |
| OW-WAR-0084 | retain explicit legacy preservation and successor mapping | OpenWarrant SDK / 1 |
| OW-WAR-0085 | replace old eleven-feature exit with SDK and CLI exit | OpenWarrant SDK / 1 |
| OW-WAR-0086 | replace compiler CLI build with provider integration contract | OpenWarrant SDK + LAMU/provider / 2 |
| OW-WAR-0087 | move document CLI parity into Phase 1 | OpenWarrant CLI / 1 |
| OW-WAR-0088 | revise ergonomic authoring consumer scope; noninteractive parity already Phase 1 | OpenWarrant SDK consumers / 2 |
| OW-WAR-0089 | revise packaging/version scope for four-phase release | OpenWarrant release / 4 |
| OW-WAR-0090 | replace Phase-2 Stable qualification with four-phase qualification | OpenWarrant release + independent reviewers / 4 |
| OW-WAR-0091 | defer Stable publication until revised hardening gate and owner release act | OpenWarrant owner / 4 |

Before implementation, inspect actual records and the current unsigned scope.
Prepare a successor for any signed contract whose scope must change. Do not relabel an old
signature as approval of this scope. No new identifiers are allocated here.
The old roadmap is historical planning evidence and no longer the target build
order for this draft. New build order is S01 → S03 → S02/S04/S05 → CLI/consumer/skill proof
→ external integration → workflow → hardening. Adoption is a qualification
requirement, not a universal start gate for prompt-only unverified work.
OW-WAR-0077/0078/0081 expose their SDK subsets in Phase 1; their provider
cases remain Phase 2. OW-WAR-0087 CLI parity precedes OW-WAR-0085 Phase 1 exit,
removing the old reversed edge. roadmap.json records required output owners.
Individual independent slices may overlap once their inputs and ownership hold.

## Other program contracts

LAMU needs its own scoped compiler/SDK integration amendment or Warrant. This
OpenWarrant draft cannot authorize edits in LAMU, KF, Katana, BLUT or Liminal.
Existing SQLite/private-memory and KF/PostgreSQL plans are unchanged; database
selection is outside OpenWarrant SDK semantics. First-party adapter readiness
must be reported per app, not inferred from this map.

## History retained

OW-WAR-0074 revision 2 binds the original RC.2 source-set acceptance extension.
The extension may be reusable, but this larger SDK amendment needs its own exact
adoption subject. Original RC.2 wire fixtures are retained byte-for-byte as
compatibility evidence. New agent-act/profile tests remain implementation work.
