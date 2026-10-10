# Native input retention candidate — 2026-10-10

OW149 and OW111 remain in progress. This candidate adds explicit
`war archive retain-runtime` requests and source-only reconnection for the
supported BLUT draft payload. The proposed ADR records the experimental format;
it does not adopt it as stable or alter a signed contract.

Three public CLI tests passed, including exact retained bytes after original
input removal and refusal of changed bytes, changed modes, extra files, symlinks,
record limits and content limits. Synthetic signature bytes remain untrusted.
The real retained BLUT CPU fixture also passed input retention; the pre-change
archive export still reported runtime-reference coverage unavailable. Source-detached recovery then passed: all required categories were retained or
explicitly absent, import/re-export was byte-identical, the known native verifier
validated the recovered public-key CPU fixture and refused changed signatures and
job bytes. Original input paths were absent and imported records remained 0600.
The archive SHA-256 is
`cf44c3f23a7a63ed52758d867b7aac009aa19fa1ee3242c455b02405e7b0e7e4`
(88,687,665 bytes). Exact data and command outputs are retained under
`/home/brianklam/Projects/OpenWarrant/docs/runtime-evidence/native-input-recovery-20261010/`.

Formatting and license checks passed. The all-feature CLI suite timed out
(exit 124) while compiling under host load. No test outcome from that attempt
is established. The detached one-worker retry passed 452 library, 3 binary and 225 integration
tests, with four ignored. Its sole failure was the progress-projection assertion:
new validation files created after compilation changed the source inventory.
The full failed output is retained compressed; no assertion has been weakened. Logs preserve
the initial unsupported-command failure, the corrected type errors, the query
failure, and the isolated-path correction; none have been erased or relabeled.

`recover-native.py` is a fixture-only recovery observation: it uses a known
external native verifier, never executes imported producer code, treats the
public fixture key as test material, checks byte-identical re-export and requires
changed-signature and changed-job rejection. It deletes only its owned source
copy after successful export and preserves the exact archive. It completed the bounded observation above. Actual KF recovery, protected custody and independent qualification
remain separate work.
