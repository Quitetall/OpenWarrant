# Provider implementation references — draft observations

Shared Warrant: `war://01a0f502-4941-70a1-a446-e1eb77dff191` (OW-WAR-0149).

These are source and PR observations from 2026-10-16. They do not select or activate an adapter revision, record participant acceptance, establish execution, or qualify a provider. The shared work order and its start conditions remain binding.

## Retained integration candidates

| Provider | Reference | Observed state | Exact head | Merge commit |
|---|---|---|---|---|
| katana | [PR 14](https://github.com/Quitetall/katana/pull/14) | MERGED | `4aad94518a6ac8ceb697c74d07c5d6d679c21bb6` | `0f5c414eab11a2c0052818b89a6fbcb1ef54d696` |
| katana | [PR 15](https://github.com/Quitetall/katana/pull/15) | MERGED | `da6553dbc8d8f3e73d09d70fecbdd677940783f5` | `7b8de536d22b58fa5a1e72bdb813a250cef40828` |
| blut | [PR 103](https://github.com/Quitetall/blut/pull/103) | OPEN | `eca4ec7db2adea3a12f76243311c643e84cbb472` | `none recorded` |
| blut | [PR 104](https://github.com/Quitetall/blut/pull/104) | OPEN | `4600e86fd7083ab0d91b44036b108f3f716f4517` | `none recorded` |

Katana’s five reported checks on each merged PR are successful. This note does not re-audit their source or extend their qualification scope. BLUT’s CLA check is FAILURE on both open PRs; its macOS compile and coverage entries are SKIPPED. The other reported checks are successful. A successful check is not a contributor agreement, participant response or production observation. This performer has neither signed nor waived the CLA.

## Local checkouts observed

| Provider | Local committed HEAD | Branch | Local changed entries |
|---|---|---|---|
| Katana | `069e08dbc2993885caa6a422c38a9a05fba316ba` | `codex/rich-ui` | 42 |
| BLUT | `6eedf207d2c539f65ef5506028d2e0e25e002e50` | `codex/openwarrant-bootstrap` | 0 |

Only committed local HEAD metadata was inspected. Uncommitted content was not used as a qualified source. Local checkout state is not proof of remote currency or of the integration candidate. No checkout, branch, working file, provider routing or service was changed.

## Acceptance and evidence still required

This document records no participant acceptance. No accepted adapter/provider tuple is inferred from a commit, PR, check name or this declaration. Before verified integration, retain the actual participant responses required by the shared work order, identify the accepted shared revision and exact provider identities, and establish the corresponding executable/verification interfaces and protected operator configuration.

The required `cross-project-qualification.md` remains absent. It must reference actual Katana and BLUT executions, original provider receipts and verification surfaces, raw observations, precise source identities, independent findings and the required authorized acceptance. Historical and synthetic observations must retain their labels. Do not create a completion report from these source observations alone.

## Observation trail

Read-only Git/PR metadata and raw check states: `/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/runtime-delivery-declarations-20261016/`. Existing implementation notes and original provider history remain in place. This is a mutable draft artifact, still `content_addressed = false`; no provenance, ownership or assurance record is minted by its existence.
