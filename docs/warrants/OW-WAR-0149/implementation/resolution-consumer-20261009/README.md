# Resolution consumer observations

The public CLI regression failed before the change, then passed. All 11
runtime-capture tests and all-target CLI Clippy passed on Rust 1.97.1. The
broader local unit run reached its 300-second deadline (exit 124) before
the last Bonsai control finished; overall result UNKNOWN. Full hosted gate
is required before merge. No tests or gate requirements were weakened.

Provider controls are synthetic. No actual native provider qualification,
signed resolution, independent verdict or human acceptance is established.
