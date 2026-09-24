---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e89-7990-8b2c-5a43577b5ce5
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The owner, 2026-09-24: *"nobody's gonna run that command to just do work
when they could just prompt or hit a jira ticket ... Reduced friction?
Compare us to jira tickets."*

A Jira ticket costs about a minute. It needs no signature, and nothing
waits on anyone before work starts. A Warrant costs a human authorization
**before** work and a human resolution **after**, two dialogs each.
Batching (OW-WAR-0072) reduces the dialogs for N acts to two per batch.
It does not remove the wait, and a batch holds one role, so the two acts
are still two batches.

For routine work the authorization tells the owner nothing new. Routine
work here means:

- a bounded edit to named code paths;
- at `basic` assurance;
- checked by the battery and by the blind verifier (OW-WAR-0117).

The authorization is still what makes the work slower than a ticket.

## Desired Outcome

The owner decides, by answering **Q-001**, how routine work is authorized:

- **A.** A standing authorization. One signature covers a bounded class
  of work. Each Warrant in the class is checked against the class and
  authorized from that signature, with an immutable per-Warrant record.
- **B.** No standing authorization. Only batching and one-click signing.
- **C.** A SAS revision that changes which acts need a human.

OW-ADR-0029 sets out the three options with the human cost of each per
routine change, and recommends A.

If the answer is A, the mechanism is built. A routine change then costs
the owner nothing before work and one line in a batched resolution after
it, which is below a Jira ticket. It keeps what a ticket does not have: a
signed authorizer of record, a battery run, a blind verification and a
human acceptance.

Under any answer these hold:

- An agent never signs, never widens its own class, and never resolves.
- No class covers an authority file (`roles.toml`, `allowed_signers`,
  `openwarrant.toml`, the SAS, the roadmap, the ADRs), a gate or its
  fixtures, or the code that decides authority.
- A resolution is always a human act.

## Non-goals

- Automatic resolution. §27.3's policy-service resolution is not enabled
  for covered Warrants, and a class cannot ask for it.
- Changing §27.2's list of human-only acts. That is option C. This
  Warrant describes C and does not build it.
- Covering anything above `basic` assurance, any `decision` Warrant, any
  Warrant carrying an ADR or accepting residual risk, or anything in
  §30.3's manual band.
- Choosing the owner's first class. The first class is the owner's to
  write and sign (STAGE-005). The draft proposes one only as an example.
- Measuring the Jira comparison with users. OBL-007 counts dialogs on a
  scratch corpus and claims nothing about time-on-task.
