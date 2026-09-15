# OW-WAR-0078 implementation

Base: 47b04783f617147d5f368eea7bbfdc84ee892737. In progress, unverified.

Public seam: validate_condition(&MetadataValue, ConditionLimits) returns a borrowed
ValidatedCondition or named diagnostic. The existing document F4 syntax checker
is shared; no second grammar, I/O, applicability or permission inference is added.
Positive limits bound aggregate condition entries and UTF-8 string bytes. Defaults
are 4096 entries and 1MiB strings. The caller already owns the supplied metadata;
validation clones no representation. Unknown operators and malformed values return
condition-invalid; exceeded limits return resource-limit.

Three public tests and eight actual-file conformance cases pass. Independent Spec
and Standards reviewers passed the syntax slice; Spec additionally exercised
Unicode, wildcard variants and path refusals in a separate workspace. Provider condition evaluation and scope fallback now pass against LAMU commit
994a693. Five condition tests plus five source compatibility tests pass, as does
Clippy with warnings denied. Integration receipts bind exact inputs and profiles.
Required UNKNOWN inclusion and dependency closure still await 0079.

Independent Spec and Standards reviews found an uncharged generated task.stage
origin. A failing public regression confirmed it; charging the origin now rejects
11 bytes and accepts the exact 12-byte result. Independent recheck passed.
LAMU commit review: PASS WITH NITS. Bounded DP allocation pressure remains a
performance consideration; invalid empty path segments are already refused.
The pinned SDK and private ValidatedCondition construction make the suggested
unknown-field panic unreachable for this revision; silently ignoring future
fields would violate semantics. Reassess that boundary with any SDK upgrade.

No participant merge, human assurance or Warrant completion is claimed.

Next stages: provider conditions (0078), selection/closure (0079), projections
(0080), packet integrity/offline export (0081), budgets/cache invalidation (0082),
and full consumer integration (0086). The latter also requires remaining Phase 1
record/legacy/CLI outputs and its exit (0083/0084/0087/0085). Each stage consumes exact
available predecessor interfaces; no missing qualification becomes a universal
prototype execution gate. Existing live/user LAMU changes remain separate.
