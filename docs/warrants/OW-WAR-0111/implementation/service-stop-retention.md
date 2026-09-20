# Unsuccessful local service attempts

The preservation integration test now runs a real failing shell command twice
and a real timed-out command twice in isolated fixture repositories. Each attempt
must retain its own run ID and records. A completed failing command must retain a
failure receipt and exact stdout/stderr. A timeout must retain an unknown run and
a blocking submission, without minting a completed-run receipt. Earlier run and
receipt bytes must remain unchanged after the second attempt.

Validation with Rust 1.97.1:

- `cargo test -p openwarrant-cli --test preservation`: 18 passed, 0 failed.
- `cargo clippy -p openwarrant-cli --test preservation -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.

These checks establish local unsuccessful-attempt retention behavior. They do not
establish archive runtime category completeness, independent verification, or
qualification. The context collector still reports non-human runtime sources as
unavailable; collecting and reconciling these records remains next work.
