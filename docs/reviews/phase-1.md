# Phase 1 acceptance

OW-WAR-0085 provides a complete assignment inventory for S01–S08, SDK-01–SDK-24, FOOTER-01–FOOTER-08 and T01–T56. Provider cases remain Phase 2; their SDK subsets are checked where required. Exact fixture IDs and public document test names are retained to detect accidental inventory loss.

Run `python3 conformance/sdk/run_phase1.py --output <new-receipt.json>` from this checkout. The runner builds the CLI and uses Cargo’s reported executable, including custom target directories. It captures source/build inputs before execution, compares them afterward, and records toolchain and executable identity. Source drift or binary drift prevents a current-host pass. A successful receipt is bounded to its recorded files and checks; it is not a Phase 1 exit or assurance mark.

Linux acceptance passed. See [receipt](../warrants/OW-WAR-0085/implementation/linux-observation.json). Native macOS acceptance with matching source inventory remains missing. Repository gate and independent review are separate observations.

Fixed skill artifacts test retained constraints, unresolved questions, stage dependencies and cycle refusal, false maturity, supplied verification identity, and additive host context pointers. They do not measure model invocation, skill activation reliability, human review time, or a real harness installation. The context-entry adapter proposes bytes only and does not establish filesystem permissions.
