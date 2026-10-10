# Current Dispatch authorization and source holders

Unverified continuation of OW-WAR-0023. The source checkpoint is `72842a20`;
`6587ec27` adapts four deliberately changed contract plants to explicit prototype
mode. The former path could emit a changed contract using an older signed subject,
or an authorized packet with floating holders. The preserved reproduction is also
in `docs/tickets/t-95254/evidence/`.

The current signed subject must equal the compiled contract digest. A changed
subject refuses before packet, context and dispatch journal writes. Explicit
prototype fallback records prototype authority rather than borrowing a signature.
Required Git source bytes must match the selected local commit. Missing observations
remain UNKNOWN; an observed byte mismatch is ERROR. Optional unestablished sources
keep empty commit pointers. A nested project cannot borrow its parent's Git HEAD.
Section pointers bind full parent bytes. Git reads reuse the bounded local reader
with lazy fetch and inherited routing disabled; no digest domain or wire schema
changes. No authority record, signature or resolved delivery manifest is rewritten.

## Observed checks

Rust 1.97.1: 12 portable CLI tests and two local Git-holder tests pass. Final
all-target CLI Clippy passes with warnings denied. The public CLI control observes
stale authorization, missing required holders, a committed-byte contradiction,
no output/journal changes on refusal, and explicit prototype fallback.

The first compile failed on the digest Result type and the current SHA-2 array's
hex formatting; the corrected build and tests pass. The initial selected battery
reported four intended context/budget refusals hidden by the new earlier authority
refusal, plus uncompiled generated-view drift. Those four deliberately mutated
fixture paths now explicitly request prototype mode; unchanged corpus positives
remain authorized. Initial failures are retained, not represented as passing.

Full repository gate and independent qualification remain separate. This work
adds admission and source-binding checks, not harness isolation, provider semantic
closure, native receipt qualification, human acceptance or Warrant resolution.

The corrected selected battery passes **52 controls, zero failures**, including
unchanged authorized Dispatch positives, context/budget refusals, the new admission
control and idempotent submit/signing fixture controls. The positive generated
corpus check also passes. No owner key or real authority act was used by the fixture.

The complete CLI library unit suite also passes: **350 tests, zero failures**.
