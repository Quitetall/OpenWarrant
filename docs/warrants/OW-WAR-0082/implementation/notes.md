# OW-WAR-0082 implementation

Unverified provider implementation: final-entry accounting, inclusive budget
refusal without truncation, full-basis cache identity and bounded single-entry
immutable reuse. Source, policy, role, selection, compiler and budget changes
invalidate reuse; earlier views remain unchanged.

T38-T41 public provider tests passed. Independent spec and standards reviews passed;
a separately retained basis digest is now charged to the serialized cache budget.
Clippy passed. Cache capacity describes retained serialized payload, not allocator
overhead; callers own their Arc lifetimes. No authority or qualification is inferred.

Provider package integration was merged without replacing either contract pointer.
Reviewer concerns about evolving struct Debug keys were false positives: the key
uses a fixed usize array. Failed measurement preserves the earlier entry.

Remaining: compatible participant merges and downstream Phase 2 exit.
