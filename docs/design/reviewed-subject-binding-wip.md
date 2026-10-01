# Reviewed subject binding: saved implementation slice

This is unfinished work for ticket t-b9610. It is saved for continuation, not ready to merge or release.

The public CLI regression reproduces an old, unbound verification being reused after its requirement changes. The first implementation slice preserves historical verdicts but excludes unbound records from current resolution and preparation. Verification requests now include the compiled contract digest and the actual declared deliverable bytes; bound responses are checked before ingestion and their subject is recorded in the journal.

The focused regression passed locally. This branch has not passed the full gate.

## Remaining work

- Test fresh bound responses, exact replay, stale contract and changed artifact refusal without writes.
- Apply the same qualification rule to status, assurance-mark issuance and acceptance.
- Check required fixture and context coverage, response obligation membership, unsafe paths and races.
- Update configured verifier contracts to echo the subject actually reviewed.
- Run appropriate checks and the full gate before proposing integration.

No human approval, independent disposition, resolution or release is recorded by this implementation slice.
