# Native Phase 1 observation, 2026-10-15

Run 38671322649 passed the existing Phase 1 inventory on native Linux x86_64
and macOS/Darwin arm64. Both receipts bind the same 12,350 source files, source
digest, case assignments and merge snapshot 1abbe1e5 (PR head cd295857).
Each has all eleven recorded checks PASS, unchanged source and stable executable
identity. The additional real FIFO recovery refusal passed on both hosts.

The raw prospective Git merge commit was retained before the PR merged. Its Git
object hash matches the snapshot; its tree equals the PR head's tree. Raw native
receipts, job/step results and logs are compressed losslessly, with hashes in the
manifest. No original receipt field was changed.

The failed predecessor run 38670200456 remains under t-b9610's native-fifo evidence.
The portable test fixture resolves that observed Apple test-build failure; it does
not fix or explain unrelated intermittent filesystem waits.

These are performer execution observations for this exact snapshot. They do not
establish independent phase review, real model/harness quality, human effort,
human acceptance, full Phase 1 exit or Stable release. The repository gate for
the final integrated head remains separately required. Later records do not
extend these observations to a different source inventory.
