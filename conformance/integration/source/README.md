# Source-provider integration

OW-WAR-0077 uses the real optional `lamu-openwarrant` crate through profile
`lamu.openwarrant.local-source/1`. The [shared contract](../../../docs/integrations/lamu-source-provider.md)
defines the exact Rust API, access basis, limits and capability declaration.
No LAMU daemon, model call or database is required.

```sh
python3 conformance/integration/source/run.py \
  --provider-root /path/to/lamu/lamu-rs \
  --output /tmp/source-provider-observation.json
```

The driver verifies the SDK Git pin, contract SHA-256 and shared fixture bytes,
then builds the real provider and runs its filesystem integration tests. It records
commands, exits, full test output, toolchain, provider HEAD/worktree status and
source/Cargo.lock hashes. Dirty candidates remain identifiable by file hashes;
final receipts must also identify committed participant revisions.

| Case | Exercised behavior |
| --- | --- |
| T11 | Capture Warrant, ADR and opaque bytes; SDK validates descriptors and exact unit ranges. |
| T12 | Rename heading/title with stable unit ID; new basis changes, old snapshot/reference remains usable. |
| T13 | Missing target, conflicting identity and altered blob digest produce distinct refusals. |
| T14 | Refuse traversal, absolute paths, symlink leaves/components and FIFO; mutation between begin/finish rejects new snapshot while preserving prior result. |
| T15 | Reverse source enumeration; exact canonical lock and explicit reference results remain equal. |
| Regression | Exhaust metadata budget before touching a later missing file. |

This profile provides sequential source rechecks, not an atomic multi-file
snapshot. Immutable checkouts or external write exclusion are required where
atomicity matters. Metadata claims confer no permission. The SDK never takes over
dependency closure, ranking, assembly, access policy or package creation.

Linux observations do not establish macOS runtime behavior or formal human
assurance. Provider-specific Linux/macOS CI is separate from LAMU's default runtime
checks. Integration is not reported complete while participant merges remain open.
