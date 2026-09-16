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


Follow-up provider revision 7ab57b5d52430f727ece3485443702a0f5ca794b fixes FIFO
fixture construction on macOS with POSIX mkfifo. Both focused Linux and macOS CI
jobs pass (ci.json). This advances the platform evidence beyond the frozen shared
contract's drafting-time statement that macOS proof was absent. The source
provider API and contract hash remain unchanged.

The portable fixture commit review passed with nits. The suggested
rustix::fs::mkfifo replacement does not exist in pinned rustix1.1.4; tests require
the POSIX utility and use unique, automatically cleaned temporary directories.
OpenWarrant's integration commit review also passed with nits: the exact dependency
pin check intentionally rejects unreviewed feature/configuration changes; added
tests must also pass because Cargo's process exit is checked; counting before
allocation is a required resource contract, not merely style.

OpenWarrant aggregate gate at 75eadb55baccfb0cd3b2d8766f8c5dd45f6330e4:
Rust1.97.1, 14/14 steps, 308 planted refusals, exit0. gate.log retains full output.
The driver also rejected altered fixture bytes with exit1 and wrote no receipt
(driver-refusal.json). Participant merges remain pending; no assurance inferred.
