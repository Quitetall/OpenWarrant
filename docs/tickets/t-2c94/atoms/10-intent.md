# 69-current: 'OW-WAR-0073's signature stands' passes if the dry run fails

Found by t-5134's performer: the negative half of this check passes when `war sign --all --dry-run` itself fails, so it can pass without checking. Require the dry run's exit status and report, as t-5134 did for the HISTORY.md check.

## Notes

- **2026-09-30 22:03 UTC, codex:** The actual signature assertion is exercised by a shell command stub, not a copied predicate. Red control reproduced exit 1 falsely passing; green controls reject exit 1 even with a success-looking report, exit 0 without a report, and the named OW-WAR-0073 refusal. Exit 0 with a success report and no named refusal passes. Controls run inside 69-current battery. Shell syntax and diff checks pass; full real-corpus battery is not yet qualified.
- **2026-09-30 22:07 UTC, codex:** Runtime 69-current plant finished: 29 pass, 2 fail. Corrected signature assertion and its controls passed. Corpus/freshness failures correspond to five generated projection drift diagnostics. Raw runtime output and exact binary provenance retained. Do not report whole plant qualified; ticket remains claimed pending repository integration checks.
