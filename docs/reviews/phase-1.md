# Phase 1 acceptance

OW-WAR-0085 provides a complete assignment inventory for S01–S08, SDK-01–SDK-24, FOOTER-01–FOOTER-08 and T01–T56. Provider cases remain Phase 2; their SDK subsets are checked where required. Exact fixture IDs and public document test names are retained to detect accidental inventory loss.

Run `python3 conformance/sdk/run_phase1.py --output <new-receipt.json>` from this checkout. The runner builds the CLI and uses Cargo’s reported executable, including custom target directories. It captures source/build inputs before execution, compares them afterward, and records toolchain and executable identity. Source drift or binary drift prevents a current-host pass. A successful receipt is bounded to its recorded files and checks; it is not a Phase 1 exit or assurance mark.

The latest recorded native run 38674510635 passed all eleven checks and the
explicit FIFO refusal on Linux x86_64 and macOS arm64. Both receipts match
all 12,363 files in immutable tree `8f68a09c`, tested merge `6c2a59bd` / PR
head `bef7e690`. Full PR gate and final candidate check passed, and that exact
tree merged as `f873ff00`; post-merge main CI also passed. See the
[complete final observations](../warrants/OW-WAR-0085/implementation/native-20261015/final-bef7e690/README.md).
Independent phase review and human acceptance remain pending. These observations
do not award a phase exit or assure later source versions.

### Earlier portability observations

Native Linux x86_64 and macOS arm64 acceptance passed in run 38671322649 at PR head
`cd295857` (tested merge snapshot `1abbe1e5`). Both receipts bind the same 12,350
source files and case assignments, with all eleven checks passing and unchanged
source/binary identities. The real FIFO recovery refusal also ran successfully on
both hosts. See the [exact observations and comparison](../warrants/OW-WAR-0085/implementation/native-20261015/README.md).
This is bounded execution evidence for that earlier snapshot. The current
repository gate is recorded above; independent phase review and human acceptance
remain separate requirements. The
[earlier Linux receipt](../warrants/OW-WAR-0085/implementation/linux-observation-v2.json)
and failed native run remain preserved.

Fixed skill artifacts test retained constraints, unresolved questions, stage dependencies and cycle refusal, false maturity, supplied verification identity, and additive host context pointers. They do not measure model invocation, skill activation reliability, human review time, or a real harness installation. The context-entry adapter proposes bytes only and does not establish filesystem permissions.
