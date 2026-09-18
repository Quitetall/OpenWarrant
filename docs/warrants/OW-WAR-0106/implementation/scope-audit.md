# OW106 scope audit (not completion or qualification)

Source candidate: 78d2371e05f475270e887659e2812ebf8ebbf0ba, followed by the
focused interrupted-adviser test and integration-only merge. Full repository gate
and latest-head CI are still pending at the time this audit is written.

| Work-order requirement | Implementation and observed evidence | Remaining limit |
| --- | --- | --- |
| Question bound to attempt/source/policy/checkpoint | hotline.question plus real HTTP clean-checkpoint test; source/policy differences and invalid checkpoints refuse | Harness containment is configured, not established by these tests |
| Authenticated retained answers | Answers store and /api/hotline routes; actor injection, stale digest, wrong credential, immutable replay and restart tests | Bearer identity is not secure human-presence proof or an assurance signature |
| Technical advice and authorized governing routing | Adviser invokes only technical/non-direct-human questions; configured exact governing scopes and direct-human role tested | Actual local/cloud model quality and trusted operator deployment not qualified |
| Unknown responder/execution does not grant permission | Missing config waits; missing answer blocks resume; timeout and live-service interruption preserve unknown, no replay launch | No unknown-writer recovery bypass is implemented |
| Resume checks current basis and old writer | Exact source/config/answer authorization/checkpoint checked under execution lock; dirty checkpoint, revoked config and changed source refuse | External/remote writer fencing remains the harness's responsibility |
| Preserve history and prevent duplicate dispatch | Immutable execution/answer/advice records, resume_from chain, concurrent resume test produces one child | Checksums detect damage; protected storage remains a prerequisite |
| Keep independent work moving | Pending question blocks a dependent while an independent Warrant completes through checks | Controller schedules whole Warrants; internal stages within one Warrant are not separately dispatched |
| Preserve time, retry and spend limits | Continuation receives remaining active time; real timeout test distinguishes fresh allowance; zero-repair continuation succeeds; hard unknown-cost cap refuses before advice launch | Paid metering is absent; free mode is an owner assertion |
| Browser and completion output | browser-observation.md shows refusal, answer without start, explicit resume, result and generated completion report | Browser walkthrough used synthetic adapter and manual adviser response; auto-routing is exercised over HTTP/process |
| Required checks and integration | adviser-all-tests.log: 89 package tests; interrupted-adviser-test.log: focused live interruption test; syntax check passes | Full repository gate and exact-head CI not yet retained as passed here |

The scope's phrase “only affected work” is proven at Warrant execution-unit
level. The wider SAS permits stages within a Warrant; that granularity is not
implemented by this reference controller. This audit does not silently redefine
that wider product requirement as complete. OW106 remains in progress until
integration and this scope boundary are reconciled with the intended workflow.

No signed contract, legacy disposition or human authority record is changed.
Synthetic observations cannot count as a real-user study or real-model benchmark.
