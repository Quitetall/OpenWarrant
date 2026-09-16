# Retained contract comparison

`war diff OW-WAR-0037 --from contract:1 --to contract:2 --json` compares
record-bound IR snapshots retained in Git history reachable from the captured HEAD.
Explicit file targets and generic proposal JSON comparisons remain supported.
A historical baseline requires an explicit `--to`; ordinary file/default diff
behavior is unchanged. Prefix a filename with `./` if it literally starts with
`contract:` and should be treated as a file.

The lookup reads the Warrant's historical authorization and generated WAR.json
at each retained snapshot, recomputes the contract digest using the compiler's
existing canonical digest implementation, and checks alias, coverage and recorded
digest. Authorization supplies the historical revision number: legacy compilers
emitted IR revision 1 even after authorization changed. The stored IR is not
rewritten. This is a read-only retained-record comparison, not validation of a
human signature, authority assignment, qualification or legacy resolution.

Conflicting digests for one revision refuse. Missing revisions, shallow history,
nonregular Git blobs, malformed records, unsupported IR fields and mismatched
digests refuse. A project nested below the Git root currently refuses this history
path. The bounded scan admits at most 1024 relevant snapshots, 4 MiB per blob and
32 MiB cumulative blob input per selector. It reads existing objects without lazy
fetch and never checks out historical files. It reports exact commit/digest origins.

For typed IR pairs, the comparison recomputes both contract digests and identifies
changed fields in their actual preimage: coverage, format basis, identity,
source/composition and relations. Noncontract execution or resolution fields are
not labeled digest causes. Generic JSON still reports structural differences
without claiming authenticated digest attribution. Existing path-amplification
limits apply before the semantic walker. Pinned show.rs bytes remain unchanged.

## Observations

Public CLI tests use disposable, explicitly synthetic Git records. They exercise
a changed title across revisions, missing history, ambiguous revision reuse,
tampered digest, unsupported typed fields, invalid selector spelling, omitted
explicit target, and a real shallow clone. The earlier JSON-layout and
path-amplification regression remains in the same public test suite.

A read-only comparison of actual OW37 contract:1 with itself found retained commit
797706e058bb053ef3a7e214ab5bf8692dd4f9fd and matching recomputed digest
 ef2591f109829dd91c51010ae702eb149cfb9c17b7e73e60b973e5cca0a4d9a0.
It reported no semantic difference. This does not invent a second actual revision.

Separate-context specification review passed the bounded implementation and
independently probed unsupported-field refusal. Standards review found a missing
single-selector restriction; the CLI now explicitly requires the target and tests
that refusal. Full gate, source commit review and final completion report follow.
