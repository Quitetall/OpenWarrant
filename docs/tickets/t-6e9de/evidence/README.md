# Budget recovery observations

Source basis fe58c3f3. Eight budget controls pass, including four new recovery
controls: held reservation and retry identity restoration; unknown cost and
policy mismatch; duplicate/out-of-order/impossible histories; preserved poisoned
admission after overcharge and integer overflow. An observed overflow cannot be
replaced by settling the same call again. All 751 core tests across 23 targets
pass with zero failures or ignores; strict all-target/all-feature Clippy and
formatting pass. Logs are retained here.

These are performer observations of pure typed replay, not independent verdicts,
cryptographic history validation, storage durability, provider charges or actual
spend-cap enforcement. History completeness, run identity, policy custody and
pre-dispatch durable commits remain the trusted host's responsibility. No paid
calls, active policy changes, signatures or legacy Warrant closures occurred.
