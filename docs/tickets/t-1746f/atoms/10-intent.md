# Detect normative rules lost under unsupported numbered SAS headings

## Notes

- **2026-10-03 23:48 UTC, codex:** Public check --generated regression reproduced silently dropped rules after recompilation on prior reader. New complementary guard uses existing section grammar, detects hidden or misattributed normative bodies, and preserves existing lettered section and section-3 behavior. Focused public CLI test and workspace all-target Clippy pass on Rust1.97.1. Two runtime plants authored; complete gate and independent qualification remain pending.
