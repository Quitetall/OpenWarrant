# W137 full-scope audit

**Runtime observations pass; Warrant remains open.** All 47 assignment controls,
five existing unassigned-verifier compatibility controls and the shared positive
corpus pass. Nine assignment unit tests pass. The
[shared 73/0 runtime log](../../../OW-WAR-0132/implementation/scope-audit-20261024/shared-controls.log)
also contains the W132 controls; they are not additional W137 coverage.

[All 13 source witnesses](deliverable-inventory.json) exist. Twelve match the
protected CI candidate `8606def37781037aa3741e8108b1d06bfb708445` exactly.
check.rs differs only by W113's rationale parsing block, outside its assignment
helper, and the current runtime controls exercise its assignment behavior.
This is bounded performer evidence, not independent acceptance.

| Scope | Observation |
| --- | --- |
| M1 / OBL-001 | Request and signature bind assignment digest; stale/edited/malformed assignment and missing role refuse; agent resolver and performer-as-verifier refuse. |
| M2 / OBL-002 | Assignment removes eligible signers, selects the sole assigned actor, refuses revoked/unassigned actors and filters both personal queue views; unassigned Warrants retain their old eligibility. |
| M3 / OBL-003 | Assigned verifier records; other actor and human without role refuse without writing the verdict; unassigned compatibility cases still pass. |
| M4 / OBL-004 | status and show count one versus two recorded fixture human identities and do not invent a review when none is recorded. Documentation and threat limits remain in TEAMS and THREAT_MODEL. |
| Pure assignment unit | CLI authorize::assignment runs all nine tests. This does not claim a public core export; core/lib.rs was not in the declared change set. |

Basis U-001 still names where assignments live as a blocking choice. Existing
code implements per-Warrant files whose digest is signed with authorization.
The inspected retained authorization responses contain no separate judgment for
that choice; their presence alone is not a decision to be invented by an agent.
An owner question is pending. No answer or past chronology is assumed.

The tests use named fixture actors, disposable keys and fixture verifiers.
They are not real developer sessions, proof of who a verifier really is, or
independent judgments on W137. Identity authentication is W138's separate scope.
The closure dry run still names missing formal requirements. Delivery history,
independent dispositions, signatures and authority were not changed.

The shared first-harness failure and its repair are described in the W132
report. Per-run scratch was removed; durable evidence is retained under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow132-ow137-20261024/`.
