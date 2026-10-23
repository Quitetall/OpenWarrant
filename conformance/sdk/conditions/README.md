# Condition syntax fixtures

Run `cargo run -p openwarrant-core --example sdk_probe -- --scope 78 --fixtures conformance/sdk`.
The driver reads each TOML fixture and calls the public condition validator. Case
IDs are labels, not dispatch keys. Eight literal cases cover accepted fields and
patterns, unknown operators, empty lists/tables, invalid types and unsafe paths.

The SDK checks syntax only. TRUE/FALSE/UNKNOWN applicability, scope fallback and
required-rule inclusion belong to the provider and are not established here.

`check_observations.py --war <binary> --fixtures conformance/sdk/conditions
--record-dir <new-output-directory>` exercises the same literals through the
public `war sdk` boundary. It retains exact requests, output and exit codes,
then checks unseen conditions and refusal to replace an existing output. The
output directory must be new and outside the protected fixture directory.
