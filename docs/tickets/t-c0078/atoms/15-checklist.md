# Checklist

- [x] Reject nonregular backup files during complete preflight; retain all current roots and marker on refusal (i-1517) — done by agent://codex, 2026-10-13: Reproduced FIFO rollback changing an earlier root then blocking. Whole-backup preflight now refuses nonregular nodes before restores. Recovery tests 3 PASS; registered recovery/idempotency controls 36 PASS, 0 FAIL; fmt, shell syntax and Clippy pass. OW130 assurance remains open; evidence implementation/recovery-node-20261013.
