# Current provider/engine runtime observation

Performer-reported, unverified Linux observation on 2026-10-09. No independent verdict, authorization, resolution or assurance mark is recorded.

## Exact combination

- Standard cookbook: merged `cbb33663f278be95aa5805b47b4f33de5ceea50a`.
- BLUT engine: current remote main `2502a4dd6385b077f21dd500851a99a6ec1795c9`, read from an isolated clean worktree.
- Rust: 1.97.1. The binary, manifest and resolved lock identities are in `build-identity.json`.
- This is a local source combination: the cookbook manifest uses an explicit path override to that exact engine checkout. It is not an untouched published binary. The original and overridden manifests and resolved lock are retained in the project evidence directory named by `source-observation.json`.

All 118 provider workspace tests and all-target Clippy with warnings denied pass for this combination. Existing provider hosted checks remain bound to their original tested candidate and are not relabeled as hosted checks of this override.

## Actual execution and refusal

An initial source-combination run also completed; its raw observations remain in the provider's `openwarrant-0047-20261009` store. The final observation below adds the explicit resource snapshot, port refusal and launch rehearsal.

The incompatible port control ran first on a disposable source copy: `STAGE-003.corpus` mapped to `war/not-a-kind`. The shipped CLI refused it by name. The original copied atom was restored; canonical signed sources were not changed. `port-control.json` and `port-refusal.json` retain the actual refusal.

For the final run, the machine's one-second CPU sample was 95.55% idle, one-minute load was 1.21 on 32 CPUs, available memory was about 26 GiB, and memory pressure `avg10` was zero. `environment-before.json` contains the actual measurements and explicit pre-run conditions. This is a start observation, not a continuous resource or sandbox guarantee.

The current OpenWarrant CLI lowered and externally checked the unchanged OW47 PlanSpec. JSON fields equal the earlier retained two-stage plan; no byte-identical formatting claim is made. BLUT's `plan run --dry-run` passed before `plan run --no-cache` executed actual job `20261009-131242-620252367`.

The jobs endpoint reports `done` and no PID. The provider reports two executed stages and zero cache hits. Both materialized and filtered files equal the original three-record fixture, whose hash matches the historical input. Actual command arguments, statuses and timings are in `commands.json`; output references and hashes are in `references.json`.

BLUT's native job files, lineage response and artifact catalog remain under the provider-owned project store. OpenWarrant retains their references and byte identities, not a second lineage stream or graph. All prior failures and earlier successful source identities remain unchanged.

## Remaining requirements

These observations address the real-run and incompatible-port evidence required by OW47/OW108. Current integration projections, record checks and full gate must still be bound to the new evidence candidate. Independent verification and human acceptance remain separate; no obligation disposition is assigned here.

OW-WAR-0149 still needs provider-owned dispatch/attempt receipt protocols and resolver integration. These native runtime queries do not substitute for that protocol, a provider seal, authenticated collector provenance or current formal runtime eligibility.
