# OW-WAR-0074 revision 2: implementation review

The amendment is signed. The source-set acceptance extension is implemented in
this worktree and awaits independent review and the remaining human acts.
Neither the Warrant nor SAS RC.2 has been accepted as completed work here.

- Base: `16f0559db9a267c2123b334224c1b1ab5e645b03`.
- Branch: `feat/ow-war-0074-source-set`.
- Authorized contract: `0e1d8a130b1015e03d7f358334b7703bc08f0e5dfb7560ec6ae52c6ed8ec618d`.
- Proposed RC.2 subject: `e003e0926b912793fa43c28b9b85cffb9f15ccaedcd99989973e2455022920d8`.
- Source files and exact digests: [observations.json](observations.json).
- Implementation diff: [implementation.patch](implementation.patch).
- Full changed files: [review-files.tar.gz](review-files.tar.gz), over the named base checkout.
- User guide: [source-set acceptance](../../../../design/rc2-source-set-acceptance-implementation.md).

## What changed

An explicit source-set proposal captures the main SAS, exact manifest, every
listed normative/reference member, the adoption decision and predecessor.
The human acceptance signs their complete subject. Unsafe or stale captures
and unauthenticated v2 responses are refused. Existing v1 commands retain their
meaning. New Warrants pin the full subject; existing authorized Warrants retain
their basis. The main-document projection reads the selected retained bytes.

Publication uses descriptor-relative no-follow opens, bounded reads, an
exclusive revision lock, complete-file publication without overwrite, and
atomic acceptance replacement. Current-source disagreement is separate from
retained accepted-capture integrity.

## Observed checks and limits

- **739 tests passed**, including 14 source-set interface tests. See [gate.log](gate.log).
- Build, formatting, required clippy, schema drift, licenses, and all 38 recorded
  attestations passed in the aggregate gate. Gate total: **12/14 steps passed**.
- The corpus step failed on six delivery drifts. Two unresolved legacy records
  were subsequently given retained artifact locations; the latest corpus check
  has **four errors**, all requiring resolved-delivery corrections. See
  [corpus-final.json](corpus-final.json).
- The planted-violation runner refused this dirty governance tree before any
  mutations. It has not supplied a passing battery result for this change.
- Concurrent-write probe observed the specific `inconsistent read` refusal on
  its first attempt and published no revision. See [changing-input.json](changing-input.json).
- Extra `clippy --all-features` found a pre-existing `items_after_test_module`
  lint in untouched `crates/openwarrant-core/src/identity.rs:207`. This is beyond
  the required gate's clippy invocation; see [extra-all-features-clippy.log](extra-all-features-clippy.log).
- Linux x86_64 observed. macOS runtime and power-loss behavior are untested.
- Performer observations do not establish independent obligations. No assurance
  disposition or completion signature was written.

## Preserved history

Authorization attestations for 0074 revisions 1 and 2 both verify. The old
literal SAS `1.0.0` remains untouched; the RC.1 designation has not relabeled its
record. The RC.2 candidate and adoption decision retain their approved digests.

For unresolved OW-WAR-0004 D-002 and OW-WAR-0063 D-001, original delivered bytes
were recovered from the base commit, verified against the original declared
digests, and retained at content-addressed artifact paths. Only the unresolved
artifact location changed. Original manifests are preserved alongside them;
producer, byte digest, authorized contract and state remain unchanged. Current
code versions are declared under 0074. See [legacy-retention.json](legacy-retention.json).

## Remaining acts

The owner reviews the outcome, implementation, evidence and remaining risks.
After review, these four exact correction requests are ready for human signing:

| Target | Changed file | Request |
| --- | --- | --- |
| OW-WAR-0005/D-001 | CLI checker | [request](correction-OW-WAR-0005-D-001.json) |
| OW-WAR-0058/D-001 | SAS record type | [request](correction-OW-WAR-0058-D-001.json) |
| OW-WAR-0058/D-002 | SAS CLI | [request](correction-OW-WAR-0058-D-002.json) |
| OW-WAR-0062/D-005 | Same SAS record type, separately pinned | [request](correction-OW-WAR-0062-D-005.json) |

Their kind is `behaviour-change`; no correction was signed by the performer.
After corrections, rerun the corpus gate and complete the protected independent
verification. Changes remain uncommitted: repository policy requires external
review for each agent commit, but the available review route can dispatch paid
fallbacks without reliable accounting. The owner's budget policy forbids those
calls. No additional paid-model calls were made. The dirty-tree plant guard
remains in force until the review/commit path is available.

The human can then accept the exact SAS request in [acceptance-request.json](acceptance-request.json).
The short local signing launcher now selects the tested source-set build for
this candidate; [launcher-preview.txt](launcher-preview.txt) is a read-only
preview. SAS acceptance will change the proposed record, so refresh its delivery
identity and evidence before final 0074 resolution. Phase 1 Warrants remain drafts.

## Independent verification inputs

The tool-created bundle is
`../../verifications/bundle-6698973947df71bd.json`. Its legacy size limit truncates
four large files; it is not a complete standalone source package. Use the full
files in `review-files.tar.gz` or the implementation worktree alongside the
bundle. The archive includes this task's changed files over the named Git base,
not build dependencies or the entire repository. Its file list is in
`review-files-manifest.json`.

Reproduce without using the performer's verdicts as independent evidence:

```sh
cargo test -p openwarrant-cli --test sas_source_set
python3 conformance/fixtures/sas-source-set/check_changing_input.py target/debug/war
cargo xtask gate
./target/debug/war sas accept 1.0.0-rc.2 --json
./target/debug/war sign 1.0.0-rc.2 --show
```

The final two commands inspect the proposal; they do not accept it. Reproduce
in an isolated checkout with the actual verifier protections required by policy.
Do not set protection flags merely to suppress the current independence warning.
