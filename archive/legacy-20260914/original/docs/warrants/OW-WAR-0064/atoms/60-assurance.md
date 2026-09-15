---
schema: oh.war/atom/v1
warrant_uuid: 01a06a12-0aa2-7503-b589-67cf75905be4
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the act is written down before it is built
- **scope:** §34.4 applied to deliverables.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** an ADR answering all four steps for a deliverable, proposed by
  the performer and adopted by the owner, and cited in this Warrant's basis by
  the time it is resolved. The basis cannot cite it at draft time, because M1 is
  what writes it; this obligation is tested at resolution, not at `war check`.

### OBL-002 — an agent cannot correct a deliverable
- **scope:** §27.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant in which an agent signs a correction, refused; the
  request half emits and writes nothing.

### OBL-003 — uncorrected drift is still an error
- **scope:** `deliverable.digest-drift`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant moving a delivered artifact's bytes with no correction
  record, still refused; and the same file with an authorized correction,
  accepted. Both, or the rule has only been loosened.

### OBL-004 — a correction record is append-only and cannot be quietly restated
- **scope:** the correction record; the two remaining plants of the six.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant editing a correction after authorization, refused; a
  plant applying a second correction of the same deliverable as an EDIT to the
  first rather than as a second record, refused. Without this, a correction
  history can be rewritten to say the artifact was always what it is now, which
  is the erasure OBL-005 exists to prevent, achieved one level down.

### OBL-005 — the superseded digest remains available
- **scope:** RQ-084; §34.4 step 4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** after a correction, the superseded digest is readable in the
  record and in `war show`; a plant that removes it from the record is refused.

## Gate Adequacy

Required at `controlled`.

**Adversarial question:** does this act make drift easier to hide? Six attacks,
each a plant that must be seen to refuse before OBL-002 through OBL-005 are
claimed: drift with no correction at all; a correction no human signed; one whose
new digest is not the file's bytes, so it corrects nothing; one whose superseded
digest was never delivered, so it supersedes nothing that happened; one edited
after authorization; and a second correction applied as an edit to the first.
The last two are the subtle pair — they attack the correction HISTORY rather than
any single correction, and a history that can be restated makes every earlier
attack recoverable.

- **outcome:** gate_added

## Residual Risk

- A correction act is only as good as the reason written in it. Nothing here
  can test whether a reason is honest; the record makes the reason attributable,
  which is a different and smaller claim.
- A human who signs corrections without reading them converts this control into
  paperwork. The two-half seam makes that visible, not impossible.
- The behaviour-change / added-refusal distinction is recorded but not enforced.
  If it should gate anything, that is a later revision, and this Warrant should
  not pretend it decided it.
