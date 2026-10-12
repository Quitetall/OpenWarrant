# Initialization JSON response

Successful `war init --json` previously printed setup messages without a JSON response. The public CLI regression failed with `stdout is not one JSON value: expected value at line 1 column 1`. The red log is retained; its binary hash was not captured.

Initialization now collects setup messages in the existing report notes and returns one `oh.war/report/v1` envelope. Human output, public initializer signatures, generated configuration and setup policy remain unchanged. The tests compare human and machine configuration bytes, require a single response on reinitialization refusal, and exercise default vibe, team, regulated, explicit-namespace program and derived-namespace program setup.

All ten CLI JSON-response controls pass. CLI all-targets Clippy with warnings denied and formatting pass on Rust 1.97.1. The observation identifies the dirty source candidate and tested binary; it does not claim an independently verified or accepted result. Full repository CI is still required on the final commit.
