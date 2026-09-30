# Condition syntax fixtures

Run `cargo run -p openwarrant-core --example sdk_probe -- --scope 78 --fixtures conformance/sdk`.
The driver reads each TOML fixture and calls the public condition validator. Case
IDs are labels, not dispatch keys. Eight literal cases cover accepted fields and
patterns, unknown operators, empty lists/tables, invalid types and unsafe paths.

The SDK checks syntax only. TRUE/FALSE/UNKNOWN applicability, scope fallback and
required-rule inclusion belong to the provider and are not established here.
