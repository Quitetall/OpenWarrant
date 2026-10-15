# Native FIFO fixture portability

Fresh native Phase 1 run 38670200456 tested exact main c8bc0cb0. Linux passed.
macOS arm64 built the SDK/CLI and passed the other recorded checks, but the
aggregate CLI test target did not compile: `rustix::fs::mknodat` is unavailable
on Apple platforms. The failure occurred before the filtered SDK CLI tests ran.
The original raw receipts are retained unchanged and compressed losslessly here.
Neither receipt establishes phase exit or human qualification.

The existing Unix batch-recovery refusal test from t-c0078 created a real FIFO
with a rustix API gated off on Apple in rustix 1.1.5. The candidate uses the same
`mkfifo -m 600` fixture command already used by the collector tests. It also checks
that the resulting node is a FIFO. All recovery assertions, the five-second bound,
and the Unix test coverage stay intact; production behavior is unchanged.

Linux: the exact FIFO refusal test and all 50 reviewed-subject CLI tests pass.
CLI all-target Clippy passes with warnings denied, Rust 1.97.1. The native workflow
now explicitly runs this existing refusal test on both hosts after its Phase 1
inventory. New native results and full CI remain pending. i-24af stays open until
native compilation and the refusal test have been observed.

`manifest.json` binds the candidate files and original raw log/receipt bytes.
The source inventory comparison is an observation, not an independent verdict.
