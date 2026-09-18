# OW47 actual BLUT execution observation

Observed 2026-09-18 from OpenWarrant `1c852b064ce20de9befb4724052bb9292c2f04a4`.
This is performer evidence, not an independent verdict or accepted reconciliation.
Historical signed atoms, judgments and deliverables remain unchanged.

The installed `blut` and `lqt` binaries expose no `plan run`. A cookbook-local
binary at `/mnt/4tb/LamQuant/training/cookbooks/lamquant/target/debug/lqt` does.
Its version is `blut 0.2.0-alpha.1+unknown`: exact bytes are hashed in identity.json,
but build-source provenance is unavailable. Do not substitute the current source
checkout revision for that binary's unknown build revision.

The existing OpenWarrant adapter emitted the original two-stage graph and BLUT
accepted it. The actual registry export is retained. Dry run exited zero. A
separate unregistered-stage probe exited one before execution; this is not the
incompatible-port control required by OBL-002 and does not replace that obligation.

Actual execution used the committed three-conversation corpus and task-owned job
and data directories, with cache reads disabled. Job `20260918-120309-976285603`
failed at `materialize_dataset_path` during artifact packaging:
`nothing to ship: no backing files under src_root` for the first stage directory.
The command exited one in 0.11 seconds; the real jobs API reports failed with no
live PID. No successful artifact receipt exists. No training or model call ran.

The installed standard cookbook source returns a DatasetJsonl handle to the
provided source path rather than materializing bytes inside the stage directory.
Current BLUT source defines the observed Empty artifact-store refusal. This is a
source-supported failure hypothesis, not proof of the unknown binary's exact
source. Do not bypass artifact checks or change the signed Warrant graph to make
the run appear successful. Reproduce with a source-pinned cookbook build, then
prepare shared BLUT/cookbook/OpenWarrant work for the materialization contract.

BLUT retains job records under `/mnt/4tb/tmp/ow47-blut-runtime-20260918`.
The lineage query was made against the real job and retained outside this Warrant;
references.json records its path and digest. No lineage graph or event stream was
copied into this Warrant. A failed-job reference does not establish successful
lineage or OBL-001 completion. OBL-001 and successful OBL-003 remain open, as do
exact source identity and independent acceptance. No historical status changed.

## Source-pinned reproduction

A clean archive of LamQuant commit 7ac7f519c92477370d735a349eec32962b0a9ab1,
cookbook subtree 43ee27189f6b817abac4f493123fe701c076fab5, was built offline with
its unchanged Cargo.lock and Rust 1.97.1. Published engine/cookbook checksums and
VCS metadata are retained in pinned-runtime-20260918/identity.json. Build passed.
The resulting binary reproduced the same packaging failure as job
20260918-120933-659122097. The real jobs endpoint reports failed, no PID.

The unknown binary is therefore no longer the only reproduction. The shared
materialization repair contract in this directory assigns provider implementation
and OpenWarrant validation without weakening engine controls. Provider repair is
being prepared on blut-backends base 12d09c5d2f00ecf52394c11657c50b14bc04bb7c;
no successful rerun or provider release is claimed yet.

## Repaired provider observation

Provider commit b10f46b (blut-cookbooks PR #1) copies source bytes into owned stage
storage, computes count/hash there and atomically publishes. Behavior schema 2
invalidates locator-only cache results; DatasetJsonl wire schema is unchanged.
The focused test failed before the repair. Afterwards all 95 provider workspace
tests, fmt and all-target clippy with warnings denied passed on Rust 1.97.1.

Using the unchanged OW47 PlanSpec, real job 20260918-121422-409152626 completed
both stages with zero cache hits. Jobs endpoint reports done/no PID. Artifact
listing names both actual outputs; owned and filtered bytes equal the original
three-record fixture. Exact binary, code-file hash, provider base/commit and
locked engine source identities are retained. The binary was built before commit;
its code-file hash was checked against the committed source. It is not described
as an untouched build from the published release.

The real lineage query remains in provider-owned task storage, referenced by path
and digest. No lineage stream or graph is copied into the Warrant. A disposable
OpenWarrant clone exercised the exact incompatible-port control: mapped input
passed, war/not-a-kind failed with STAGE-003.corpus named, original fixture restored.
Canonical signed atoms were never edited. See repaired-runtime-20260918/.

Provider PR: https://github.com/Quitetall/blut-cookbooks/pull/1 . Hosted run
35343738646 did not start either job because GitHub reported failed account
payments or a spending-limit restriction. This is unavailable CI, not a code
failure or pass. Integration/hosted validation and independent acceptance remain
open; no historical verification or resolution was changed.
