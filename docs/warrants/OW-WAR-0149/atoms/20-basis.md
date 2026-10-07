---
schema: oh.war/atom/v1
warrant_uuid: 01a0f502-4941-70a1-a446-e1eb77dff191
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Shared sources and participant boundaries

- Relevant legacy SAS sections: §47, §48.1–48.5, §49.2–49.3 and §65; OW-WAR-0026, OW-WAR-0027, OW-WAR-0047 and OW-WAR-0108. Live `war sas status` reports 1.1.1 accepted and 1.2.0 proposed. The current generated normative projection reflects the proposed 1.2.0 document; generation does not accept that revision or replace an existing signed contract. This shared draft likewise does not activate new provider rules.
- OpenWarrant cbe9a8a8: `crates/openwarrant-core/src/seam.rs` defines KatanaReceipt and BlutLineageReceipt. These validators check minimum fields and references, not a provider-authenticated seal. `resolve.rs::runtime_receipts_match_the_basis` has no connected store.
- Katana checkout 0b0ac9dd1cbf69a2628ea214a4e841c5bd2888e1: `crates/katana/src/exec.rs` returns status, session path, answer, tool summaries and token usage. `crates/katana-mekugi/src/lib.rs` owns an append-only event log and BLAKE3 chain. These interfaces do not supply the complete OpenWarrant KatanaReceipt contract. The scoped Rust search found no dispatch_digest, prompt_ir_digest or receipt_digest producer.
- BLUT checkout 6eedf207d2c539f65ef5506028d2e0e25e002e50 (rechecked 2026-10-01): `src/framework/lineage.rs` provides read-only job lineage; job/status/artifact APIs own their observations. LineageNode now carries portable input/output content identities alongside preserved legacy hashes. A scoped Rust search under src found no dispatch_digest, receipt_digest or warrant_uuid producer. This checkout differs from the OW47 runtime engine pin. Do not silently attribute its behavior to the old run. No complete dispatch-bound provider receipt has been observed.
- Retained OW47 successful run: job 20260918-121422-409152626, provider b10f46be930be8f2696a35941fe36a2d7c2ab7c7, engine ffecee56abc87175a55dedc9f92d3537fa5a4227. References and actual failures remain in `../OW-WAR-0047/implementation/`.
- Provider PR Quitetall/blut-cookbooks#1 remains open at aebd937beb6fee46664ca7bdd691bec3e732a3d3 with the latest visible hosted checks failed on 2026-09-18 (fmt/clippy/tests and secret-scan). Those historical failures do not establish their causes or qualify a newer checkout. The old runtime binary path is now absent. Retained records do not prove a fresh rerun or merge.

## Blocking unknowns for verified integration

1. Each provider must publish its receipt schema, exact digest/seal calculation and supported verification interface. An OpenWarrant checksum of stored bytes is not that provider seal.
2. Katana must supply its own PromptIR identity, effective confinement/capabilities and terminal receipt. OpenWarrant must not reconstruct them from a transcript.
3. BLUT must provide a stable job-to-dispatch binding and a way to check that status, artifacts and lineage references belong to that job. A matching path or ordinary JSON file is insufficient.
4. Approval of the cross-project adapter version and source identities remains required where the participating Warrant or repository policy requires verified start.

Preparation and prototype tests may proceed under the owner prompt. These unknowns block claims of provider qualification and verified integration, not ordinary drafting.
