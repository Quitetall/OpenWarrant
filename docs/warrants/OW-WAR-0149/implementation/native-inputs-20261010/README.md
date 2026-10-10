# Native input retention candidate — 2026-10-10

OW149 and OW111 remain in progress. This candidate adds explicit
`war archive retain-runtime` requests and source-only reconnection for the
supported BLUT draft payload. The proposed ADR records the experimental format;
it does not adopt it as stable or alter a signed contract.

Three public CLI tests passed, including exact retained bytes after original
input removal and refusal of changed bytes, changed modes, extra files, symlinks,
record limits and content limits. Synthetic signature bytes remain untrusted.
The real retained BLUT CPU fixture also passed input retention; the pre-change
archive export still reported runtime-reference coverage unavailable. Full
source-detached native recovery remains pending at this checkpoint.

Formatting and license checks passed. The all-feature CLI suite timed out
(exit 124) while compiling under host load. No test outcome from that attempt
is established. A bounded detached retry uses one build worker. Logs preserve
the initial unsupported-command failure, the corrected type errors, the query
failure, and the isolated-path correction; none have been erased or relabeled.

`recover-native.py` is a fixture-only recovery observation: it uses a known
external native verifier, never executes imported producer code, treats the
public fixture key as test material, checks byte-identical re-export and requires
changed-signature and changed-job rejection. It deletes only its owned source
copy after successful export and preserves the exact archive. It has not yet
completed. Actual KF recovery, protected custody and independent qualification
remain separate work.
