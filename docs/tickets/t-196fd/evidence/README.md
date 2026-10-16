# Run-budget SDK observations

Source basis e803da28; pure in-memory accounting only. No paid model call,
operator enrollment, active policy edit, hard billing enforcement or independent
verdict. Four public controls cover aggregate reservations across callers/retries,
estimate/unknown refusal under a ceiling, missing settlement retaining reservations,
explicit undefined-cap unknown accounting, actual overcharge poisoning admission,
duplicate settlement and integer overflow. All 747 core all-target/all-feature
tests pass, zero failures/ignores. Core strict all-target/all-feature Clippy and
formatting pass. Logs are retained here.

The trusted host must authenticate bounds/charges, protect one run-wide policy
and ledger, serialize and durably persist reservations before execution, restore
unsettled reservations on recovery and check effective policy. This SDK object
observes no such protections and cannot establish them by naming a variant.
Actual Claude cost estimates/threshold do not qualify. Provider/host integration
and entire-corpus independent/human qualification remain open.
