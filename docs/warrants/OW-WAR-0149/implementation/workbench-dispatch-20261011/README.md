# Workbench Dispatch context bridge

Unverified performer observations. No independent verdict, human acceptance or
release qualification. Implementation source: `4459e9cd3bb6869484f34e702142a72125cc5030`.

The opt-in draft v3 execution config maps a workbench source and stage to an
exact core Warrant UUID, contract digest and stage. The actual SDK compiles and
retains the Dispatch, creates its portable context bundle and validates it.
The worker receives original bundle bytes through bounded gzip/base64 transport.
Run records retain both identities and the exact Dispatch/bundle binding.
Existing execution v1/v2 interfaces and portable bundle format are unchanged.

Public CLI tests exercise matching subject compilation and wrong UUID/digest
refusal, with unchanged journal and no emitted packet, manifest or retained
Dispatch on refusal. The bundle module's 13 integration tests passed before the
wrong digest control was strengthened to use a correctly shaped raw digest.
The final strengthened run also passed all 13 tests.

HTTP/process tests passed for delivery to a synthetic worker, its actual offline
SDK validation, changed core-contract refusal, and bundle mutation after SDK
validation. The mutation test first failed: the worker launched. The adapter now
hashes the exact read bytes before encoding and refuses that mutation. A portable
bundle remains context, not execution authorization or assurance.

Strict workspace/all-target/all-feature Clippy and four roadmap tests passed.
The first full Python run finished 222 tests with five errors: HTTP disconnect/
timeout, teardown timeout and Git status timeout. It did not pass. The affected
four test methods and all three context methods then passed together (seven
tests, 2.927 seconds), with unchanged assertions and deadlines. A complete
223-test rerun exited 124 at its 600-second deadline, after additional errors;
it did not establish a full pass. One owned fixture service was observed in
`D` state at `jbd2_log_wait_commit`; this establishes a journal wait at that
moment, not the sole cause of all errors. No assertion or deadline was weakened.
Hosted web checks must pass before merge. Schema drift checks also passed.

The dependency schema pack merged in PR213 at
`fc4a14d11ba8208f90fefa8ba531fbcb01a651ac`, after its required CI gate passed. Earlier CLI argument/compiler errors and malformed config
failures remain retained. One earlier HTTP connection ended before the tested
link; it is an unavailable integration observation, not a failed link assertion
or an established diagnosis. A later positive process test passed with a stable
copy of the actual CLI executable.

Scope remains incomplete: changed resume/repair workspaces need fresh core
bindings; provider captures are not automatically attached to workbench attempts;
real providers, custody, spend, confinement and independent qualification remain
required. OW149 stays in progress. OW95 has no real participants and stays pending.

Compressed logs retain original bytes and hashes in `logs.json`. Synthetic
fixtures create no authority, participant consent or signed completion records.
