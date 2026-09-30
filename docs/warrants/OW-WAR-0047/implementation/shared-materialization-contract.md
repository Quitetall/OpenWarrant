# Shared OW47 materialization repair contract

Status: prompt-authorized implementation proposal, unverified. This single adapter
contract links OpenWarrant OW-WAR-0047 and the BLUT standard cookbook repair. It
does not amend signed OW47 atoms or accept either project's release.

## Required outcome

The original OW47 PlanSpec must execute its two named stages and produce actual
status, artifact and lineage records. `materialize_dataset_path` must return
owned, stable dataset bytes inside its stage directory so BLUT can package its
output. Keep argument shape, stage name, DatasetJsonl wire schema, traversal
refusal and engine portability/integrity checks. Do not make external files
portable by weakening artifact ownership checks.

## Participant responsibilities and start requirements

- Standard cookbook: copy the selected source into stage-owned storage, compute
  count and content identity from that copy, and publish it atomically. Failed
  reads must preserve previous output. The source must remain untouched. Bump
  stage behavior schema so old cached locator-only results cannot masquerade as
  materialized output. Cover source deletion/mutation and failed publication.
- OpenWarrant: retain both original failures; rebuild a pinned cookbook with the
  repair, run the unchanged signed graph, compare corpus bytes and observed
  result, and keep lineage in BLUT with exact references. Successful fixture
  execution is not assurance or stable integration qualification.
- BLUT engine: existing artifact ownership, packing and content checks remain
  binding. No engine code change is proposed by this contract.

Preparation and isolated tests may proceed independently under the owner's prompt.
Cross-project runtime validation requires the fixed cookbook source identity,
exact engine dependency identity and unchanged OW47 PlanSpec. Release/publishing
and secure human acceptance remain separate acts. No model calls are required.

## Evidence and handoff

Original reproduction: runtime-20260918/. Source-pinned reproduction:
pinned-runtime-20260918/. Provider base: blut-backends
12d09c5d2f00ecf52394c11657c50b14bc04bb7c; published failing cookbook 0.1.0
source c3746059708c89fa1cad0c6b3c74cc28367fbd3c. Both retain the same locator-only
materializer. Track fixes by exact commit and public-seam observations, not claims
that a passing unit suite satisfies the whole Warrant.
