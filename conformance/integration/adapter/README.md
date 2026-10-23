# Real provider and SDK exchange

Run the provider profile with the published dependency intact:

```sh
python3 conformance/integration/adapter/run.py \
  --provider-root /absolute/path/to/LAMU/lamu-rs \
  --output /absolute/path/to/new-observation.json
```

The driver checks both the declared SDK pin and Cargo's resolved dependency graph
before provider tests or compilation. The `openwarrant_core` dependency of the
specific provider manifest must resolve to the agreed Git repository and exact
revision. A path patch or a different revision refuses; changing the manifest
alone cannot make an override qualify under this profile. The successful receipt
retains Cargo's actual metadata and resolved package identity. This is build
provenance, not signature authentication or proof against a malicious toolchain.

The exchange runs the six provider profiles, adapter controls, deterministic
package generation, a separate SDK-only consumer and record/preservation controls.
Package integrity does not establish semantic coverage or human assurance.
Missing executable or metadata observations do not establish a passing profile.
The driver never starts another compiler or model as fallback.

The fast external-protocol and output-retention controls run with:

```sh
python3 -m unittest discover -s conformance/integration -p test_observation_output.py
```

For the real override refusal, prepare an explicit Cargo path patch and its lock
file in a disposable provider checkout, then run:

```sh
python3 conformance/integration/adapter/check_resolution.py \
  --provider-root /absolute/path/to/disposable/LAMU/lamu-rs \
  --record-dir /absolute/path/to/new-refusal-observation-directory
```

This checker first confirms the actual path override through Cargo metadata. It
requires the public driver to refuse it without publishing an observation. It
changes no provider configuration. Preserve the original configuration and lock
file when preparing the disposable checkout. A failed run retains its actual
output; a before-fix acceptance stays a failed control, not a qualification.
