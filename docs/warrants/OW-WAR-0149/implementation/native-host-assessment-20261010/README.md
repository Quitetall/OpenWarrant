# Protected native Katana assessment

`war runtime assess <alias> --selection selected.json --native-store /absolute/operator/store --json` opts into native Katana verification. Without that option, missing native verification stays UNKNOWN. This is a local host configuration interface, not a new standard authority act.

The store must already hold authenticated active authority and a signed, operator-activated collector enrollment. `runtime-host.json` binds the actual execution UID, canonical repository root, repository identity and collector. `runtime-native.json` is a separate bounded, operator-owned regular file. Both configurations remain inaccessible for writing by the execution account, and their exact bytes are checked around native execution. These checks are not an atomic launch fence.

The native configuration schema is `oh.war/runtime-native-config/v1-draft.1`. Its `entries` array contains at most 256 exact dispatch selections. Each entry has:

- `dispatch_digest`: exact retained dispatch digest.
- `provider`: explicit `kind`, `identity` and `version` matching the capture and active enrollment.
- `executable`: absolute independently approved verifier path; enrollment supplies its authenticated SHA-256.
- `event_log`: absolute `path`, independent bare lowercase `sha256`, and positive finite `max_bytes`.
- `trusted_log_head`: independently retained native BLAKE3 head.
- `authorized_capabilities`: explicit permission upper bound.
- `confinement_required` and `hard_spend_cap_required`: explicit booleans.
- `timeout_ms`: 1–60,000; `max_response_bytes`: 1–4 MiB.

The configuration is strict JSON: unknown fields and duplicate keys are refused. Log snapshot budgets total at most 128 MiB per assessment. The log is captured into a non-executable sealed Linux image and held through verification. The CLI does not derive a key, head, permission or expected input digest from a receipt. There is no mutable input or unprotected configuration fallback.

Missing exact dispatch policy stays UNKNOWN. A superseded attempt, provider disagreement or malformed configuration is refused. BLUT directory-shaped job custody is explicitly unavailable in this configuration version; its independently configured SDK adapter remains separate. Kernel/platform protection, real provider cost accounting and confinement may remain unavailable. A required observation cannot be replaced by a declared success.

The command records no authority activation, human acceptance or assurance. Production host accounts, a populated actual Katana provider roundtrip, BLUT custody, workflow integration and independent qualification remain required observations. Fixture signatures and namespace identities are not real human approval.

## Observations

The CLI test first failed on the missing `--native-store` option. The completed runtime regressions passed 26 tests, with two opt-in native-provider tests skipped. Collector loading/signature regressions passed seven tests, with two namespace entries skipped in that normal run. The explicit two-account namespace test separately passed: duplicate protected policy keys refused; populated CLI policy reached the activated sealed fixture executable and rejected its invalid response; changing protected spend policy invalidated a held verifier before launch. All-feature/all-target CLI Clippy with warnings denied, fmt and whitespace checks passed.

This populated fixture uses a signed, selected `/usr/bin/true` image that produces no valid native receipt. It proves fresh native refusal through the configured CLI path, not an actual Katana receipt acceptance or a production human act. Original RED, final observations and source hashes are retained beside this note.

Generated-document compilation and check completed: 2,028 passes, 625 warnings, zero errors and zero unknowns. This checks record/projection integrity and runs no execution gate. Protected-main CI remains separate.
