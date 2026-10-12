# OW130 retry evidence

The existing independent OBL-002 finding identifies a skipped required before-fix control. This supplement executes that bounded historical comparison. It does not change that finding, an assurance disposition, or an authority record.

Both binaries were built with Rust 1.97.1 from clean checkouts: `9c3c9150264f8b023e67e1035760521aeb518e58` before the change, and `4433ee361fa4d2c78f621515c81a6832e6dcfdd9` after. On the same scratch Warrant and literal v1 fixture response, the former rewrites the record and exits 1 with “already journalled”; the latter exits 0 with `verify.replayed`, preserves the record modification time, and leaves the complete Warrant file-content tree and journal unchanged. The fixture payload comes from the after-fix commit's `69-idempotency.sh`.

The current plant now binds a v2 response to a reviewed subject. Feeding it unchanged to the old binary would test schema refusal instead of historical retry behavior. The standalone comparison uses the historical v1 payload intentionally; it does not claim the current plant ran with a historical backend. Reproduce with `python3 historical-retry.py BEFORE_BINARY AFTER_BINARY` from this evidence directory; temporary fixtures are confined to `/mnt/4tb/tmp/openwarrant-ow130-retry-fixture` and removed by the runner.

The separately retained clean `d5a47d74` observation passed 62 selected verifier, idempotency and dispatch-budget controls. Its eight source hashes still match this branch. Those controls used the scratch-helper prelude, not the full registered battery. This historical comparison closes an observation gap for reassessment; it is not independent verification, a gate receipt, human acceptance, or completion of all OW130 scope.

The first attempt to create the historical checkout timed out before Git registered it. Its partial build was stopped and discarded. The clean builds and results above come from the recreated checkout, not that incomplete attempt. A second successful comparison added the whole-Warrant tree check.
