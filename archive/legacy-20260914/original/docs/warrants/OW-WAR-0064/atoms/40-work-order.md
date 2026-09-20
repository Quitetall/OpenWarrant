---
schema: oh.war/atom/v1
warrant_uuid: 01a06a12-0aa2-7503-b589-67cf75905be4
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. An ADR stating the correction act: what it is, what it supersedes, what it
   preserves, and why superseding beats regenerating. §34.4's four steps,
   answered for a deliverable.
2. A correction record — `corrections/<deliverable-id>-<n>.toml` under the
   Warrant whose deliverable moved, schema `oh.war/correction/v1`, the same
   on-disk shape as `resolution.toml` — carrying at minimum: the deliverable id, the superseded
   digest, the new digest, the reason, whether the change is a behaviour change
   or an added refusal, the authorizing human and role, and the effective time.
   Append-only; a second correction of the same deliverable is a second record,
   never an edit.
3. `war correct <alias> <deliverable-id>` as a two-half seam: the agent emits a
   request, a human's `--response` is ingested. An agent-signed correction is
   refused, exactly as `war resolve` refuses one.
4. `war check`: `deliverable.digest-drift` stops being raised for a deliverable
   whose current bytes match an *authorized* correction record, and is still
   raised for every other drift. Three new refusals, each distinct and each
   separately planted: a correction whose NEW digest is not the file's current
   bytes (it corrects nothing); a correction whose SUPERSEDED digest was never
   the deliverable's recorded digest (it supersedes nothing that happened); and a
   correction signed by an agent.
5. Six plants: drift with no correction record; a correction signed by an agent;
   a correction whose new digest is not the file's bytes; a correction whose
   superseded digest was never delivered; a correction edited after
   authorization; a second correction of the same deliverable applied as an edit
   rather than as a second record.
6. The superseded digest stays readable in the record and in `war show`. §34.4
   step 4 preserves the original; a correction that erases what it replaced has
   done the thing this act exists to prevent.

## Frozen Surfaces

`deliverable.digest-drift` as an error for uncorrected drift. §56.2 resolution
records. The thirteen §56.1 requirements. No assurance atom of any other Warrant
is edited to make a verdict easier.

## Premade Instructions

- Every plant asserts the rule AND a detail string, and must be seen to fail
  before it is trusted.
- The three defects that motivated this Warrant are NOT fixed here. Fixing one
  would make this Warrant its own first user, and a correction act whose first
  exercise is by its own author is untested where it matters.

## Autonomy and Escalation

Tier T2. The design of the act is a normative decision and escalates: the ADR is
proposed, not adopted, by the performer.

## Rollback

Revert. Nothing is corrected until a human signs a correction, so a reverted
implementation leaves the corpus exactly as it stands today — including the three
unfixed defects.
