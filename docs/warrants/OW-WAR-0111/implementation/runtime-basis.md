# Archive-derived runtime query basis

`war archive runtime-basis ARCHIVE --json` validates the experimental archive,
reconstructs its source IR and returns `result` with schema
`oh.war/runtime-archive-basis/v1-draft.1`. The result names the archive digest,
Warrant identity, recomputed current contract digest and retained authorization
record revision/digest/source tuples. Contract-history coverage remains explicit.
No provider process runs, no signature is inferred and no archive coverage is
upgraded. This is a comparison input for provider readers, not trusted authority.

The source-detached integration test checks exact IR contract identity, unchanged
archive bytes, preserved unavailable-history state and rejection of a substituted
archive subject. All 16 preservation integration tests pass on Rust 1.97.1, as does
all-target CLI Clippy with warnings denied.

Applying the command to the retained source-complete fixture exposed a provider
fixture mismatch: the UUID agrees, but the provider test used
`integrity.composition_revision_digest` as `contract_digest`. OpenWarrant's actual
`WarIr::contract_digest()` is a distinct digest over its defined contract view.
Existing retained provider observations remain unchanged and prove byte
preservation only; they do not establish this cross-system contract binding.
The provider fixture must use the producer's recomputed contract digest.

Correction observed under KF `cb7442541dbf8103ced99fca540289d9599fbe42`, PR6.
A new real PostgreSQL/MinIO round trip passed after the old binding failed the
producer-digest comparison. Its signed package and fixture public key are retained
in `provider-correct-contract-package.tar.gz`; private keys are not retained.
The offline reader ran from an empty temporary directory with an unusable database
URL and preserved the producer's contract digest. Missing trust refused with empty
stdout. Exact digests and revision limits are in `provider-correct-contract-proof.json`.
Both provider revisions carry the correct digest, but provider revision 2 still
reuses source revision 1's IR. No distinct local revision or runtime execution is
inferred. Historical packages remain intact.

### Provider consumer reconciliation

KF PR6 merged at `6ea034037224d779b0b35aaa7d841e98905ec22a` after exact-head
CI run 35376677769 passed. The next adapter is KF PR7, commit `d69e8181cca5ccd91867d4d178ad4b1c4aea7395`.
It consumes the producer result through `--archive-basis`, authenticates the
provider snapshot with external public keys, and compares revision plus digest.
The real corrected package matched source revision 1; provider revision 2 stayed
explicitly without retained source. Equal digests do not erase that gap.

Retained output, observation identity and 45-test export suite log accompany this
record. Built CLI ran from an empty directory with an invalid database URL.
Type checking, focused lint, package build and measurement generation passed.
PR7 full CI remains pending at recording time. Source basis authenticity remains
a producer/caller responsibility; the provider adapter does not claim it.
No independent disposition, signature, stage coverage or completion is granted.

### Source-bound stage query inventory

The runtime query basis now includes current and retained historical stage graphs,
exact source digests, manifest locations and unresolved references. The source
fixture yields two distinct declarations with no unresolved reference, but this
is not runtime coverage: `execution_coverage_established` remains false.
History availability is reported separately. No source graph implies that its
stages ran, that a provider receipt exists, or that an actor had permission.

Sixteen preservation integration tests passed on Rust 1.97.1. Source-detached
query tests compare graph content and exact bytes; historical tests require each
retained commit's declaration independently. Existing forged-subject/history
refusals remain intact. All-target CLI Clippy with warnings denied passed.
