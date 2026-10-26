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

## Opt-in real local model control

`katana_real_local.py` uses a separately prepared local-only LAMU service with
exactly one declared local model. It does not manage the service or modify global
routing. This is a separate experiment; the scripted producer and its deadline
remain unchanged. Declare the service's memory, CPU and lifetime limits before
starting it, and run the LAMU/Katana skill's HTTP preflight successfully first.

Supply explicit private `LAMU_SETTINGS`, `LAMU_REGISTRY`, `LAMU_DB` and XDG paths,
preserving the real HOME. Set `OW_KATANA_LOCAL_URL` to its `http://127.0.0.1:PORT`
origin, `OW_KATANA_LOCAL_MODEL` to that model, `OW_KATANA_CLI` to the actual CLI,
and `OW_KATANA_NATIVE_VERIFIER` to the provider's built verifier. No credentials
or proxy routes are forwarded. Then run from this checkout:

```sh
OW_KATANA_REAL_PRODUCER="$PWD/conformance/fixtures/runtime-capture/katana_real_local.py" \
OW_KATANA_REAL_RETAIN="$EVIDENCE/captures" \
cargo test --locked -p openwarrant-cli --test all \
  runtime_capture::real_local_model::actual_model_edit_receipt_matches_recorded_dispatch_and_refuses_stale_basis \
  -- --ignored --exact --nocapture
```

Set `EVIDENCE` to an absolute durable evidence directory first. The producer
retains both successful and failed attempts there. Its coding loop uses four
turns, four tools, 150 seconds, builder policy and bubblewrap. It must correct
one arithmetic function without changing the two tests or Makefile. The host
checks exact expected bytes before running those tests, then the control verifies
and imports the actual dispatch-bound native receipt. Empty capability allowance
and changed task basis must refuse. Required confinement and accounting remain
UNKNOWN; resolution cannot gain authorization or independence from this capture.

This proves the bounded local integration, not model quality across tasks,
protected production custody, sandbox enforcement, dollar accounting, provider
agreement or independent/human acceptance. The retained log head is explicitly
test instrumentation. A turn marked `halted` or an unchanged function fails this
control even if the CLI writes a native receipt.
