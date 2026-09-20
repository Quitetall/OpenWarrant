# Standalone integration gate

Source: `90b1eaef297aa2519e01849037ca8a89890ed8f6`. Rust 1.97.1. `cargo +1.97.1 xtask gate` exited zero:
14 steps and 308 planted controls passed. The standalone clone remained clean.
The complete log is retained as full-gate-90b1eae.log.gz. This gate contains the
shared contract and initial runtime evidence; later provider-test evidence was
committed separately and is not silently attributed to this source revision.

The clone has independent Git refs. Its build-cache directory is shared only with
the earlier task-owned gate cache, with no concurrent Cargo writer. No user's
working checkout or service was reset. This is performer-observed gate evidence,
not an independent Warrant verdict, provider hosted CI or human acceptance.

Provider CI remains unavailable because GitHub would not start its jobs due to
account billing/spending limits. PR #120 is stacked on PR #118 until parent merge.
OW47 signed records remain unchanged. Integration completion is not claimed.
