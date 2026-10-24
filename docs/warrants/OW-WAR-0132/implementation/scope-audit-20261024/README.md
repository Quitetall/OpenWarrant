# W132 full-scope audit

**Runtime observations pass; Warrant remains open.** The current candidate
passes 20 hotline controls plus the shared positive corpus. Seven performer
unit tests pass. All ten declared delivery files are byte-identical to protected
CI candidate `8606def37781037aa3741e8108b1d06bfb708445`, whose 14-step gate and
1,711 plants passed. This is performer evidence, not an independent verdict.

The [shared runtime log](shared-controls.log) has 73/0 total: 20 hotline,
47 assignment, five unassigned-verifier compatibility controls and one positive
corpus check. [Source witnesses](deliverable-inventory.json) cover all ten
deliverables; no delivery pin was refreshed.

| Scope | Observation |
| --- | --- |
| OBL-001: stop affected stage only | Blocking question prevents actor spawn; other stage runs under --all; nonblocking question does not block; fixture human answer reopens the stage. |
| OBL-002: no responder stays UNKNOWN | Both frontier and next name question.no-responder; agent answer refuses and preserves question bytes; restoring a fixture human removes the finding. |
| OBL-003: unknown spend and cap refusal | Every observed ending reports unknown spend. Zero-spend mutation is detected; mandatory cap refuses before compilation/spawn; absent opt-in refuses and explicit opt-in runs. |
| OBL-004: repair/recovery accounting | Failed/answered sequences enforce distinct caps; failure consumes no repair; lowering repair cap fences recovery and raising it permits the same recovery. |
| Defaults, docs and AM-002 | Config comments/code retain three repairs, two recoveries and explicit unmetered consent. HOTLINE documents limits and mid-run questions; next/init and cancellation/token fixtures are covered by the exact-source protected gate. |

The missing item is not another coding step. STAGE-001 explicitly calls for the
owner's defaults decision. Inspected retained authorization responses have empty
judgment lists; the only retained submission is for STAGE-004, the battery.
This does not prove no decision ever happened. It means these records do not
establish it. The owner already chose three repairs in chat; confirmation of
two recoveries and unknown-cost opt-in is pending. No answer is assumed.

The closure dry run, existing independent records and signed history remain
unchanged. A chat choice would record direction now, not fabricate a past human
stage or secure signature. Formal delivery/gate reconciliation, independent
qualification and human resolution remain separate.

The first private harness failed before controls because it passed an absolute
tool path to plants that prefix REPO_ROOT. Correcting the private harness fixed
that setup; no production code was changed. Per-run scratch was removed; durable
raw evidence remains under `/mnt/4tb/artifacts/OpenWarrant/keep/ow132-ow137-20261024/`.
