# Bind provider runtime receipts to actual dispatch and compilation basis

Current resolve.rs treats every Katana/BLUT stage as unmet because no store/import is wired. Existing KatanaReceipt and BlutLineageReceipt are typed minimum records, not verified provider receipts. Use shared provider contracts; preserve actual external authority, exact source identities, lineages as references, and all signature/independent gates. No placeholder receipt hashing or existence-only success. Prompt-authorized unverified implementation; no paid model calls.

## Notes

- **2026-10-01 01:13 UTC, codex:** Shared draft OW-WAR-0148 (renumbered OW-WAR-0149 on 2026-10-07: its alias collided with the signed typed-records Warrant) is the single cross-project contract. Current Katana exec/log interfaces and BLUT lineage/job interfaces lack a complete observed dispatch-bound receipt protocol; provider seal/schema and explicit participant revision responses remain open. Former OW47 runtime binary is absent now. Preserve historical actual run, do not pretend a fresh rerun or provider merge.
