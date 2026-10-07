# Verification records must not make evidence stale

t-22fd excluded authority records from a receipt's whole-tree binding but kept verification records bound. The blind verifier reads receipts (the bundle) and writes verifications/; that write then stales every tree-bound receipt, and re-recording the evidence changes the receipts the verifications were made against — a loop with no fixed point. It also forces document.review@1.0.0 (which needs established independent verifications) to come after verification, while its own receipt is evidence. Verification records (and bundles, responses under verifications/) are judgments about the evidence, not source: exclude them like authority records, stating the limit (a gate that reads verifications says nothing about ones written after it). Found on the 2026-09-26 evidence run.

## Notes

- **2026-09-26 03:53 UTC, claude:** Paths excluded and why: RESOLVING.md 'A verification does not move the tree'. war verify (request) and war document review write nothing; --response files outside verifications/responses/ and CLAUDE_VERIFIER_LOG inside the tree stay bound (outside the tool's reach).
