# Runtime capture controls

`control.py` tests local capture retention without a native provider.
`katana_cli_fixture.py` is an optional producer for the existing native capture
control. It drives the real Katana one-shot CLI through two scripted localhost
responses: read `input.txt`, then finish. It is a test fixture, not an agent
backend or execution admission mechanism. It needs no model service or API key.

Build Katana's CLI and native verifier from an explicit provider checkout:

```sh
cargo build --locked --manifest-path "$KATANA_ROOT/Cargo.toml" -p katana \
  --bin katana --example openwarrant_verifier
```

From an OpenWarrant checkout, select those exact built artifacts and run:

```sh
OW_KATANA_CLI="$KATANA_ROOT/target/debug/katana" \
OW_KATANA_NATIVE_VERIFIER="$KATANA_ROOT/target/debug/examples/openwarrant_verifier" \
OW_KATANA_FIXTURE_PRODUCER="$PWD/conformance/fixtures/runtime-capture/katana_cli_fixture.py" \
OPENWARRANT_NO_PROJECTS=1 \
cargo test --locked -p openwarrant-cli --test all \
  runtime_capture::actual_katana_turn_roundtrips_through_capture_and_current_resolution \
  -- --ignored --exact --nocapture
```

Use Cargo's reported paths when a custom target directory is configured. The
fixture checks absolute input, output and CLI paths. It isolates HOME/config/cache,
clears API keys and proxy settings from its child, and sets an explicit localhost
provider URL. The CLI has a 25-second process limit inside the existing control's
unchanged 30-second producer deadline. Optional `OW_KATANA_CLI_RETAIN` names a
directory for positive raw captures before the consumer's negative mutations;
without it, ordinary test cleanup removes the temporary corpus.

The fixture's event-log head is test instrumentation, not protected custody.
Native cost and confinement remain unknown. The consumer rejects excess
capabilities, changed logs, a wrong head and false assurance, and preserves
UNKNOWN when a mandatory observation is absent. This proves transport and
bounded local execution, not real coding-agent behavior, provider agreement,
human acceptance, independent verification, a phase exit or an assurance mark.
