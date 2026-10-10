# Runtime selection transport and authenticated HTTP assessment

Source implementation: `e1b84dd74fe51a9210bae1680bbcb6fb2a74af77`.

The public SDK now offers bounded `parse_selection` without repository access. Its file reader retains its original path semantics. CLI `runtime assess --selection -` and resolution dry-run `--runtime-selection -` accept the existing selection schema on stdin, capped at 16 KiB. Duplicate fields and oversized input refuse; future schemas remain UNKNOWN. No repository input file is required. Native signature, receipt and hash domains are unchanged.

The separate reference app adds authenticated `POST /api/runtime-assessment`. Clients supply only a corpus Warrant alias and explicit capture selections. Native policy can be configured only at service startup. The service invokes the real CLI with bounded stdin, output and time, preserves its report and returns matches, unknown or refused with `qualified: false`. Wrong sessions, foreign origins, client policy fields and malformed selections refuse. An executor-writable native store produces the actual CLI refusal. Missing captures remain UNKNOWN. This assesses retained corpus captures, not workbench draft attempts; it cannot sign, activate, merge or resolve.

Twenty-eight runtime regressions passed (two ignored native tests); actual Katana and BLUT two-UID namespace fixtures were run separately and passed through both file and stdin transports. Four collector regressions passed (two ignored). Rust 1.97.1 formatting and all-feature/all-target Clippy with warnings denied passed. All twelve HTTP workflow tests passed in the final run.

Earlier HTTP attempts failed on session startup and the existing board's 10-second SDK timeout. A syscall trace observed a session-file fsync taking 12.158996 seconds; the parent app also failed readiness on this machine. The test now observes durable readiness for up to 30 seconds and cleans up its own child on failure. No durability write was removed; the service request deadline remains 10 seconds. Earlier failures are retained alongside the final passing suite. These results do not establish a startup latency or availability service level.

These are local CPU fixtures and public test keys, not human acceptance, production custody, independent verification, spend accounting or confinement qualification. Native HTTP positive qualification is not claimed; native matching is exercised by CLI namespace fixtures, while HTTP covers missing evidence and protected-store refusal. Connecting draft execution attempts, native human-resolution request/ingestion and real-user qualification remains. OW95 has no participants yet, per the owner's decision.

`observation.json` records original log hashes. This report is performer evidence only and leaves OW149 in progress.
