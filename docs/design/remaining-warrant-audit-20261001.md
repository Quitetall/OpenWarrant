# Remaining Warrant preparation audit

## Latest read-only observation

Observed from the reviewed-subject implementation branch at source 7af33955 with the uncommitted actor/Warrant matching correction. The production binary includes that correction. `war prepare --all --dry-run --json` selected 115 Warrants: all are planned, none is proved prepared or complete. No gate, model, verifier or signature was executed. The raw report is `/mnt/2tb/ow-post-binding-prepare-dryrun.json`.

All 115 verification steps are now planned; none is reported current. Earlier output reported 26 as current, but that output counted history without a binding to the reviewed subject. The corrected classification preserves the old records and refuses to treat them as current independent assurance. It does not turn old observations into failures or prove that the delivered software is defective.

Counts overlap: one Warrant can have several unmet requirements.

| Unmet requirement | Selected Warrants |
|---|---:|
| artifact digests verify | 84 |
| every required obligation is dispositioned | 115 |
| every required gate has admissible result | 115 |
| independence requirements are met | 115 |
| no required unknown remains | 87 |
| no blocker remains | 87 |
| required deliverables exist | 54 |
| runtime receipts match the basis | 3 |
| required judgments exist | 56 |
| residual risks have sufficient authority | 56 |
| exact authorized Contract Revision | 21 |

The report lists 21 human authority acts. Runtime receipt matching remains unmet for OW-WAR-0045, OW-WAR-0047 and OW-WAR-0071. Shared draft OW-WAR-0148 (renumbered OW-WAR-0149 on 2026-10-07: its alias collided with the signed typed-records Warrant) records the provider/interface work needed; retained OW47 execution is historical evidence, not a fresh dispatch-bound receipt. Required real-user qualification needs actual consenting participants.

The configured independent verifier invokes a paid-capable Claude command without reliable aggregate spend accounting. The owner's configurable $10 hard-cap default remains binding. Do not launch a real corpus sweep until reliable accounting or an explicitly permitted backend meets that policy. A green planning envelope is not a passed completion gate.

The subject-binding implementation remains draft PR #138. Full context/evidence/input closure, canonical bundle binding, race controls, the final full gate and independent qualification remain open. The dry run does not establish readiness to merge the draft or resolve a Warrant.

## Consolidation evidence

Consolidation PR #136 merged as 68cb4eca1c91e17f545761bdc7fcdaab750c7d5a. Hosted run 36809646825 passed all 14 gate steps and all 1,277 plants; its merge-candidate check passed 29 findings with no warnings, unknowns or errors. This proves that exact consolidation candidate, not the later subject-binding branch or all remaining Warrants.

The earlier local cbe9a8a8 candidate passed 14 gate steps and 1,277 plants on Rust 1.97.1. Its first hosted run 36799088658 cancelled at the then twenty-minute job bound. That cancelled observation remains history; it was followed by the bounded CI fixes and successful run above. No required-check bypass was used.
