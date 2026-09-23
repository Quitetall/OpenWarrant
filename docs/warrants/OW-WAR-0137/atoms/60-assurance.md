---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6056-70f1-a94b-56ec20257a39
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the assignment is what the authorizer signed, and a later edit is refused
- **scope:** `assignment.rs` and `authorize.rs` on a scratch program with
  two humans and one agent in `roles.toml`. No claim about repositories
  sharing assignments any other way.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the authorization request lists the assignment and its digest;
  - a response echoing a different digest is refused
    `authorize.stale-assignment`, and nothing is written;
  - an assignment edited after authorization is reported by `war check`
    by name;
  - an assignment naming an actor without the role is refused
    `assignment.role-missing`;
  - naming the agent as resolver is refused by kind;
  - naming the performer as its own verifier is refused.

### OBL-002 — assignment narrows who may sign and never widens it; each person sees their queue
- **scope:** `sign.rs` `eligible` and `choose_actor`, `war sign --list
  --as` and `war inbox --as`, on the same scratch program.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - with Ada and Ben both resolvers and Ben assigned, `war sign OW-X --as
    Ada --dry-run` is refused `sign.not-assigned`;
  - `war sign OW-X --dry-run` with no `--as` selects Ben;
  - an assignment naming Cy, who holds no role, makes nobody eligible;
    nobody else is added;
  - `war sign --list --as Ada` omits OW-X and `--as Ben` lists it,
    identically in `--json`;
  - a Warrant with no assignment lists both, as before.

### OBL-003 — on an assigned Warrant, only the assigned verifier's verdict is recorded
- **scope:** `verify.rs` ingest on the scratch program. No claim about
  who a verifier really is; that is OW-WAR-0138.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a response from the assigned verifier is recorded;
  - one naming another actor is refused `verify.not-assigned`, and no file
    appears under `verifications/`;
  - a human verifier without the `verifier` role is refused
    `verify.role-missing`;
  - on an unassigned Warrant the same responses behave as before this
    Warrant (`84-verify-bundle.sh` still passes).

### OBL-004 — no view claims two-person review that did not happen
- **scope:** `war status --json` and `war show` for the scratch Warrants.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - a Warrant whose authorizer, verifier and resolver are one human reports
    one distinct human;
  - with two distinct humans on record it reports two;
  - no field says "reviewed" when no verification record exists.

## Gate Adequacy

Required at `controlled`: this Warrant changes who may sign an act. The
load-bearing obligation is OBL-002's "never widens". If an assignment could add
an actor to `eligible`, it would be a grant, and a grant written by whoever
edits a file under `docs/warrants/`.

**Adversarial question:** could an agent assign itself as verifier, or
reassign resolution after the human signed? It could only by writing the
assignment. Before authorization, the human reads and signs the list. After
authorization, the digest no longer matches (OBL-001). An agent named as
resolver is refused by kind whatever the file says.
