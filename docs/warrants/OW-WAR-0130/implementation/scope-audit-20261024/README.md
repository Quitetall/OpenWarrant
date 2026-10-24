# W130 scope audit and future-manifest correction

**Implemented correction, unverified.** The registered idempotency/recovery/
compatibility plant first passed 32/0 against the combined current source. The
completion audit then found that manifest parsing happened before major-version
negotiation. Valid TOML declaring `oh.war/manifest/v9` with future fields failed
as a malformed current manifest instead of reporting UNKNOWN.

The correction inspects the exact already-read manifest bytes for a recognized
newer major before the current typed reader. It emits `compat.newer-record`
and returns no validated manifest or Compilation Basis. It does not translate
future fields or authorize execution. Known versions retain the existing parser
and its refusals. The shared schema scanner supplies the header interpretation;
known manifests retain its byte-scan fast path.

The corrected [before run](future-manifest-red-corrected.log) has 33 passes and
the intended failure. The [final run](future-manifest-green-corrected.log) passes
34/0, including positive corpus, the original controls and two new public-seam
controls. The future record stays byte-identical, emits UNKNOWN without ERROR,
and is not reported `manifest.valid`; malformed current input still fails with
its original `cli.error`, missing uuid and exit 1. Eight compatibility units,
nine Preflight CLI tests, all-target warnings-denied Clippy, fmt and Bash syntax
pass. Exact source/tool hashes are in [observation.json](observation.json).

Earlier failed test attempts are preserved: one expected the wrong existing
current-parser diagnostic/exit, and a first-green assertion accidentally rejected
valid sibling Warrants. The retained fixture report shows zero errors for the
future subject. Neither test defect is presented as another production defect.

## Full scope inventory, retrospective

[All 13 declared deliverables](deliverable-inventory.json) exist and have current
source witnesses. These hashes are observations, not refreshed delivery pins.

| Requirement | Current source and observation | Limit |
| --- | --- | --- |
| Durable batch marker and backups | batch_cmd.rs copies and fsyncs files/directories before atomically publishing the marker. Registered SIGKILL after 2/3, interrupted check/new batch refusal, full listed-path recovery, refusal history, unwritable-target rollback and successful 3/3 controls pass. | No power-loss-during-recovery or concurrent-writer guarantee is inferred. |
| Shared replay result | journal_cmd.rs returns Recorded/Replayed, compares actor identity and checks the existing fact before append. | Current shared work-event encoding is distinct from historical v1 evidence. |
| Authorization ingest | authorize.rs checks the prior journal before record writes; equivalent retry and actor conflict controls pass. | Other sign-off kinds are not claimed by this authorization-only ingest proof. |
| Question act | questions.rs checks before asking; same-input retry and actor conflict controls pass, while a different question is recorded. | No claim over every unrelated command. |
| Submission act | run_cmd.rs checks before submission writes; retry preserves the full Warrant content tree and actor conflict preserves journal length. | Existing compilation and dispatch checks remain prerequisites. |
| Verifier-response ingest | verify.rs plans responses and checks existing facts before writes; retry/conflict controls pass. | A fixture verifier is not independent proof about W130. |
| Evidence attachment | evidence.rs checks a currently admissible recorded run before re-executing; retry/conflict controls pass. | Stale/failed runs are new attempts, not equivalent retries. |
| Version negotiation | compat.rs, config.rs, repo.rs and COMPATIBILITY.md implement requires_war, named newer-record UNKNOWN and current parser refusals. The --recover/exit-2 CLI cases from AM-002 pass. | No translator for unknown schema fields or families is claimed. |

This inventory is written during the completion audit. It does not establish
that STAGE-001's inventory existed before implementation. The source now uses
`work.idempotency-conflict`, with the old name retained in its explanation;
the original contract named `journal.idempotency-conflict`. Versioned v2 work
events also coexist with legacy readers. These changes are disclosed, not used
to rewrite or silently reinterpret the historical contract.

The actual historical before/after v1 verification retry proof remains in
[retry-proof-20261012](../retry-proof-20261012/README.md). This run deliberately
leaves IP_WAR_BEFORE unset: feeding today's v2 response to that old binary would
exercise schema refusal instead of retry behavior. The skipped comparison is
not counted as a passing current control and the earlier independent finding
is not cleared by this performer report.

W130 remains in progress. Formal current-delivery/gate reconciliation,
independent reassessment and human close-out remain separate. No old signatures,
contract bytes, assurance dispositions, delivery pins or record schemas changed.
The private scratch repositories and fixture state were removed; durable
observations remain under `/mnt/4tb/artifacts/OpenWarrant/keep/ow130-scope-20261024/`.
