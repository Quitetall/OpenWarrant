---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-6056-70f1-a94b-56ec20257a39
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The register already names more than one person. `docs/authority/roles.toml`
holds actors, kinds and roles (§27.4), and `war sign` computes who is
eligible for each pending act (`sign.rs` `eligible`, `choose_actor`).
What a team needs beyond that does not exist:

- **No assignment.** Every eligible human sees every pending act. Nothing
  records "Ada reviews OW-X, Ben resolves it". With two eligible signers
  `choose_actor` only says "pass --as".
- **No personal queue.** `war sign --list`, `war inbox` and the web UI's
  Queue show one list for everyone. Nobody can ask "what is waiting on me".
- **The verifier is self-declared.** `war verify --response` accepts any
  `verifier.actor` that differs from the performer (`verification.rs`
  `admissible_for`). It does not check that the verifier holds the
  `verifier` role in `roles.toml`, or that it is the reviewer anyone chose.
  In a team, "Ada verified this" is a string anyone can write.

## Desired Outcome

- **Assignment is a record, bound by the authorizer's signature.** A Warrant
  may name who verifies it and who resolves it. The authorization request
  lists the assignment and its digest, the way it already lists the
  deliverable set (THREAT_MODEL row 12). The human who authorizes signs who
  reviews. A change after authorization is a named finding.
- **Assignment narrows, never widens.** An assigned actor must already hold
  the role in `roles.toml` and be eligible by kind. An assignment can only
  remove other eligible signers from an act. It never grants anything.
- **A queue per person.** `war sign --list --as <actor>` and
  `war inbox --as <actor>` show the acts that actor may take now, with
  assigned acts first. `--json` carries the same list.
- **A verdict names a verifier the register knows.** On an assigned Warrant,
  ingest refuses a verdict from anyone but the assigned verifier. A human
  verifier must hold `verifier` in `roles.toml`.
- **Views stay honest about review (§27.4).** No surface says "reviewed by two
  people" unless the records show distinct human actors in those roles.

## Non-goals

- Authenticating who a person is beyond what `--ssh-sign` already checks.
  That is OW-WAR-0138 (`OW-PHASE-9/authentication`).
- Signed human verdicts. Recording a human's verdict under their key is
  OW-WAR-0138's seam. This Warrant checks the verifier against the register
  and the assignment.
- The web UI beyond this machine (OW-WAR-0139). The web UI's Queue reads the
  same queue function; it gains the filter without a change of its own.
- Notifications, email, chat or any server. The shared queue is the
  repository, shared through git.
- Changing who may sign an act by kind (§27.2). Agents stay refused.
- The independent verifier itself (OW-WAR-0117).
