# Protected executable observation — performer evidence

The reference capture module can acquire an absolute, protected ELF path against an independently approved SHA256 digest, seal its observed bytes in a Linux memfd, and run that image with a cleared environment and bounded deadline/output. No mutable command or descriptor escapes the public interface. Linux executable memfd and seals must be available; unsupported platforms or kernel observations remain unavailable. Scripts are refused because their interpreters are outside this image binding.

Acquisition rejects symlinks, parent traversal, self-owned or writable path components, writable/nonregular/nonexecutable descriptors, files over 128 MiB, noncanonical digests and mismatched bytes. Effective-account access is checked; unexpected access-check errors are unavailable. Execution allows 1–300 seconds and 1–16 MiB response budgets.

## Observed controls

- Two unit controls execute `/usr/bin/true` from the sealed image, observe write refusal, and refuse mismatched digests, relative paths and invalid execution budgets.
- The provisioned namespace control uses operator UID 0 and capability-free worker UID 1. The operator replaces an approved `true` image path with `false` after acquisition. Execution through the path observes failure; execution through the sealed image observes success. A fresh acquisition refuses the replacement. Writable files, symlinks and scripts are refused.
- Existing four crypto controls pass; two namespace entries are explicitly ignored in that default run and the namespace control is then run separately.
- All 444 CLI unit regressions pass. Strict all-target/all-feature CLI Clippy passes.

These are performer observations, not independent verification or human acceptance. The expected digest must come from approved configuration. This component does not authenticate a caller, prove operator trust, protect ELF interpreters/shared libraries, qualify sandbox or spend accounting, or fence a native launch against a simultaneous authority revision. Native adapters are not automatically wired to it yet. The namespace uses software fixture signatures, not real operator enrollment.
