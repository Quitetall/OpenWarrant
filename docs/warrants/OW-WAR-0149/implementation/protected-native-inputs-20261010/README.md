# Protected native verifier data

`ProtectedInput::acquire` captures an operator-owned regular file into a bounded Linux memfd with write, growth, shrink and execute seals. The expected SHA-256 must come from independently approved host configuration. A digest provided by a receipt or an archive manifest is not approval. This helper does not authenticate configuration, execute data, grant a permission or establish assurance.

The caller sets a positive finite byte limit, capped at 128 MiB. The source and every ancestor must be protected from the actual execution account; links, self-owned paths, writable paths, non-regular files, over-budget bytes and mismatched content are refused. Unsupported platforms/kernel protection remain unavailable. There is no unsealed temporary-file fallback.

`argument()` provides an explicit procfs path to the retained parent descriptor. The object must stay alive through verification, and the verifier must be able to read the parent's procfs descriptor. The descriptor has CLOEXEC: unrelated child processes do not inherit an open handle. This is data transport in a supported process domain, not a sandbox or a portable on-disk authority record. Keys, plans and logs can use it; an archive's producer executable is not run by this helper.

## Observed controls

On Rust 1.97.1, the existing explicit two-account namespace fixture passed with operator UID 0 and unprivileged executor UID 1. A child read the exact original bytes after the operator changed the original inode in place and replaced its path. The executor made the private image mode-writable, then an actual descriptor write and truncation still failed. Adding execute permission failed. Writable-source, symlink, incorrect digest and byte-budget controls were refused. Seven normal collector-loading/signature regressions passed; two namespace entries were intentionally ignored in that normal run and the namespace case was invoked separately. CLI all-feature/all-target Clippy with warnings denied, fmt and whitespace checks passed.

The original RED, final controls and source hashes are retained here. Software fixture keys and namespace identities establish no production human act. Native input selection must still come from protected host configuration and be wired into actual adapter/CLI assessment. Job-directory custody, provider confinement/spend observations, protected-main CI and independent qualification remain separate.

Generated-document compilation completed. The first bounded check timed out after 240 seconds without a verdict under observed host I/O pressure: UNKNOWN, not a source failure. A subsequent bounded check completed with 2,028 passes, 625 warnings, zero errors and zero unknowns. Its full record-only report is retained as `protected-input-generated-check.json.gz`; this does not establish execution gates or independent qualification.
