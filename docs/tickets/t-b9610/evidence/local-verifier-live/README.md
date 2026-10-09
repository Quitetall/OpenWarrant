# Real local verifier refresh control

This is a synthetic scratch corpus reviewed by a real local model. It proves
the bounded public CLI refresh behavior below. It does not qualify production
code, the general accuracy of this model, full context/custody closure, or the
Katana/BLUT receipt protocol. No production disposition or human act was made.

- `first.json`: the configured client recorded the actual model's verdicts.
- `stale-refusal.json`: after `local-review-control.txt` changed from alpha to
  beta, the old response was refused as `verify.subject-stale`. The harness
  compared the verification files and journal before/after: no writes occurred.
- `refreshed.json`: a second actual model review was ingested for the changed
  snapshot. Exact old record bytes remain under the corpus verification history.
- `summary.json`: assertions from the live harness, including the visible-text
  claim established and the absent production execution evidence not established.
- `model-runs/`: exact bundle bytes, client source, system prompt, raw provider
  output, response TOML, reported token usage and timing for both reviews.
- `before-refresh/` and `old-response.toml`: retained earlier bytes.
- `corpus/`: the complete final scratch corpus, preserving its original layout.
- `live-control.py`: the exact harness source used for this passing run.
- `runtime-observation.json`: observed local process identities and executable
  file hashes. This is not a provider-authenticated attestation.
- `files.sha256.json`: local SHA-256 identities of the copied run files, before
  this explanatory README was added. These are byte checksums, not signatures
  or replacements for OpenWarrant's canonical packet identity.

The original run directory was `/mnt/2tb/ow-local-verifier-live-final2`. Files
were copied unchanged; absolute paths inside the captured inputs remain as
they were during execution. Do not rewrite them to make the record relocatable.
Run the documented harness with a new directory to repeat the experiment.

The provider was LAMU 0.6.0 (`2a26299`) with the explicit local
`qwen3.6-27b-uncensored-heretic-v2-q4_k_m` model and a local llama-server process.
The inspected installed-source HTTP route dispatches to local backend ports.
No paid model call was made. The client does not meter monetary cost and reports
it as unknown, preserving token usage separately.

The preceding attempt is retained separately under
`../local-verifier-preliminary-self-stale/`. Its model answered, but logs were
inside the checked source tree, so ingestion correctly refused the stale
subject. The final client refuses that configuration before writes or inference.
That preliminary run is not counted as a passing control.
