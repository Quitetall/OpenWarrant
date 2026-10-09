# SDK calculation observations

Rust 1.97.1. `cargo test -p openwarrant-core --test derived_metrics` first failed on the absent SDK operation. The implemented operation passes five public API tests: exact noninteger and large-count values, observed zero versus missing inputs, zero denominator, invalid names/populations/sources/subset counts, all eight fixed names, millisecond conversion and wrong-unit refusal. Focused Clippy with warnings denied passed. An intermediate test compile failure after adding measurement units is retained; the missing test field was corrected before the final passing run.

Sources and populations are caller-supplied labels, not authenticated observations. Tests use synthetic inputs. No real-session timing, new historical baseline, improvement, signed Warrant completion, human act or independent assurance is claimed. CLI/report integration and full batch gates remain pending.
