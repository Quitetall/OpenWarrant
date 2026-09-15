# Source provider review

Independent Spec and Standards reviews inspected the real LAMU provider in separate
scratch workspaces. Initial review found aggregate descriptor metadata was checked
after subsequent I/O. A public reproduction with two bounded metadata records and
a later missing file observed `target-missing` instead of a resource refusal.

The repair adds borrowed-label preflight, cumulative raw/descriptor accounting
and unit quotas before further source traversal. Both reviewers reproduced
`resource-limit` after the fix. The Spec reviewer also independently observed
same-byte inode replacement refusing with `source-changed`. Five provider tests
passed on Rust 1.97.1. Both final verdicts: PASS for the bounded provider scope.

Standards review also confirmed cached unit indexes/locks remove repeated corpus
parsing and the dependency ADR/optional CI retain the default Rust 1.89 runtime.
These reviews do not confer human assurance or prove atomic snapshots, full
context compilation, macOS runtime behavior or participant merge state.

The committed provider is 9ff093b75439c9a7f674260ebddba20aee01fbef. See
observation.json for exact profile, source hashes, SDK revision, fixture test
output and toolchain. The publication branch excludes unrelated unpublished
LAMU platform-proof work. Default runtime tests passed in an isolated configuration
without provider API keys; existing fmt and Clippy checks failed in unchanged files.


LAMU commit review returned PASS WITH NITS. Its read-loop/path findings retracted
themselves. The alleged unit-quota underflow was independently disproved through
a public probe: units=1 with the three-unit fixture returns resource-limit before
subtraction, without panic. No changes were made for those false positives.

Participant pull request: https://github.com/Quitetall/lamu/pull/2 (draft; existing
baseline lint failures remain). This is the actual published provider revision,
not a simulated response adapter.
