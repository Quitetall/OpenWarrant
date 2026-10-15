# Final native and repository observations

PR #284 source `bef7e69086711a686fcc74a053f6d2cb6e239216` passed native run
[38674510635](https://github.com/Quitetall/OpenWarrant/actions/runs/38674510635)
on Linux x86_64 and macOS arm64. Both receipts bind all 12,363 source files to
tested merge snapshot `6c2a59bd5aedb4daf1ec8d2a1bf5519a30c9e32b`, tree
`8f68a09c300abe5f653f53ce811de5475090f6e9`. Each host passed all eleven
checks and the explicit real FIFO recovery refusal. Before/after inventories
match the immutable Git tree; case assignments match between hosts.

The full PR gate and final candidate check passed in run
[38674510650](https://github.com/Quitetall/OpenWarrant/actions/runs/38674510650),
within the finite 75-minute job limit (57m03s job interval). The exact tested
tree merged as `f873ff0051c458d534a9f64a9f48a9d1f8e05cdc`. Post-merge main
run [38679538732](https://github.com/Quitetall/OpenWarrant/actions/runs/38679538732)
also passed (47m58s job interval); its PR-only final candidate step was
correctly skipped on the push event. The original 60-minute cancellation
remains in t-66d2f's runtime-20261015 evidence.

The compressed receipts and logs retain their original decompressed bytes.
`sha256.json` hashes the retained files; the CI log hash receipt separately
binds its original raw bytes. Commit objects bind the tested and merged
snapshots; `completed-integration-proof.json` records their exact identity.

These are performer observations. They do not establish independent phase
review, human acceptance, a Warrant resolution, a release or an assurance mark.
They make no execution claim about later source revisions.
