# Independent review observations

Source reviewed: 4c8d573 (full identity recorded in completion report).

Specification reviewer: PASS. Both public CLI tests passed; independent disposable-Git probe refused unsupported typed IR fields even against a generic JSON target. Digest attribution roots match WarIr::contract_digest preimage. No scoped defect found.

Standards reviewer: initially found omitted --to routing ambiguity. After explicit paired-selector documentation/refusal, independently reran both public tests (including omission and shallow clone): PASS on Rust 1.97.1. Pinned show.rs unchanged from 2dc1a258.

Local LAMU review_commit: PASS WITH NITS. UTF-8 truncation suggestion is a false positive: oversized output refuses before text conversion, and lossy decoding would weaken evidence handling. Snapshot overflow is already bounded before use; reviewer acknowledged correct rejection. Repeated prefix allocation is a minor bounded performance nit, retained to keep this repair focused. No independent gate disposition, human signature or qualification is minted by this review summary.
