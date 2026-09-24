---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e89-7990-8b2c-5a43577b5ce5
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the decision is the owner's, and it is recorded with its options
- **scope:** `docs/adr/atoms/OW-ADR-0029-standing-authorization.md`,
  `questions/Q-001.toml` and, under A only, the SAS 1.2.0 proposal
  (`docs/sas/revisions/1.2.0.toml`). No claim about any code.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - The ADR states A, B and C, each with its human cost per routine
    change (dialogs per round, reads per change, whether work waits),
    the recommendation and its reason.
  - The Decision section matches the answer in Q-001, and `answered_by`
    is a person.
  - Refusal: until Q-001 is answered, `war frontier` shows STAGE-002
    to STAGE-005 blocked, and the ADR stays `status: proposed` with no
    Decision text.
  - An independent reviewer confirms that nothing in the ADR removes an
    act from §27.2's list under A. Under C, the reviewer confirms the
    ADR says plainly that §27.2 changes.

### OBL-002 — a Warrant inside the class is authorized from the owner's signature; one outside is refused and nothing is written
- **scope:** `war standing apply` on a scratch corpus in
  `69-standing.sh`, with one signed class (fixture signer key) and
  planted Warrants. No claim about classes or contract fields the
  fixtures do not use.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - A Warrant inside the class gets an `authorization.toml` with:
    - its own exact contract digest;
    - the class signer as `authorizer`;
    - `policy_basis = "standing://<id>@<rev>"`;
    - its declared deliverable set.

    `war check` passes that record.
  - Refusals: each of these planted Warrants is refused with its own
    term named, and the Warrant's directory is byte-identical before and
    after:
    - a declared path outside the class's globs;
    - `assurance_level = "controlled"`;
    - `profile = "decision"`;
    - an `adr` atom;
    - an `accepted_residual_risk` assumption;
    - an obligation citing a gate not in the class;
    - a stage over the class's `budget_tokens` or `wall_time_seconds`;
    - an application after `expires_at`;
    - an application past `max_warrants`.
  - After the class is revoked, a Warrant that was covered before is
    refused with `standing.revoked`.
  - A covered record whose Warrant is then edited outside the class makes
    `war check` report `standing.outside-class` (error), not `PASS`.

### OBL-003 — a class that names or matches an authority path is refused
- **scope:** `standing.rs`'s parser, `war standing propose` and the
  class's dry-run signing, over the fixture classes in `69-standing.sh`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Refusals: each of these fixture classes is refused at propose
    and at `war sign --dry-run` with `standing.never-coverable`, naming
    the path:
    - a class naming `docs/authority/roles.toml`;
    - a class naming `docs/authority/allowed_signers`;
    - a class naming `openwarrant.toml`;
    - a class naming `crates/openwarrant-cli/src/sign.rs`;
    - a class whose glob `**` or `docs/**` would match any of those.
  - The never-coverable set is a constant in `standing.rs`. The plant
    also greps for any file read that could extend it, and fails if one
    exists.
  - A class limited to `crates/openwarrant-cli/src/tui/**` is accepted.
    This is the positive control: the refusal is not a refusal of
    everything.

### OBL-004 — a class cannot be widened except by a new human signature, and a wider class does not re-cover the past
- **scope:** the class record, its signature, `war check`'s
  re-derivation and the signing path, on the scratch corpus.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Refusals:
    - Edit one glob in a signed class file. `war check` reports
      `standing.unsigned`, and every `war standing apply` under it is
      refused.
    - A class file with no signature covers nothing.
    - `war sign standing:<id> --as <an agent in roles.toml>` is refused
      by kind, whatever roles the agent holds.
    - No MCP tool accepts, revokes or ingests a class.
      `war mcp --describe` lists the refusal.
  - A new revision signed by the human covers Warrants applied after it.
  - A Warrant authorized under the earlier revision keeps that record,
    and its `policy_basis` names the earlier revision (§31).

### OBL-005 — a resolution is never automatic
- **scope:** `resolve.rs` and the signing path for a covered Warrant with
  every obligation established by a blind verification, on the scratch
  corpus with `policy.allow_automated_resolution = true` and a
  `policy_service` actor in `roles.toml`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Refusals:
    - The policy service's resolution of the covered Warrant is refused
      with `resolve.standing-needs-human`.
    - A class file carrying any resolution term is refused as an unknown
      term.
    - `war resolve <alias>` alone writes no resolution.
  - The same Warrant is resolved by a human in a `war sign --batch` with
    two other covered Warrants: one batch document, one signature.

### OBL-006 — routine work never takes a file from work in flight
- **scope:** `ownership.rs`'s rule for covered Warrants, on the scratch
  corpus. The rule is as Q-003 decides.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Refusal: a covered Warrant declaring a path currently owned by an
    authorized, unresolved Warrant is refused with
    `standing.in-flight-owner`, naming the owner.
  - A covered Warrant declaring a path whose owner is resolved takes
    ownership as OW-ADR-0021 describes, and the earlier pin reads
    `deliverable.superseded-by`.

### OBL-007 — the dialog count is what OW-ADR-0029 claims
- **scope:** three covered routine Warrants on the scratch corpus, taken
  from class signature to resolution, with the fixture signer counting
  `ssh-keygen -Y sign` calls. No claim about human time-on-task or any
  real user.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Signing the class takes 2 signatures, once.
  - Before work, the three Warrants take 0 signatures.
  - Resolving the three in one batch takes 2 signatures.
  - Refusal: the same three as ordinary Warrants under B take 4
    signatures and cannot start before the first batch. The plant
    asserts both counts, so a regression that re-adds a pre-work
    signature fails it.

## Gate Adequacy

Required at `basic`. OBL-003 and OBL-004 carry the most weight. If a class
could reach an authority file, or grow without the owner signing again,
the standing signature would be a blank cheque, and every other control
would rest on a record the agent could rewrite. Each of those claims is
paired with a planted refusal, and each refusal checks that nothing was
written.

**Adversarial question:** can an agent widen its class without touching
the class file? It could do so by editing the code that checks the class.
That code is in the never-coverable set, and `check.rs`'s call into it is
held by `69-standing.sh`, which is also never-coverable. The plants run
in the battery before the owner accepts anything.

**Second adversarial question:** does A let the owner sign something they
did not read? The owner reads the class once. The signing screen prints
every glob with what it matches today, and the bounds (expiry and count)
limit how long a class drawn too wide can last. Each change is still read
once, at acceptance, after the blind verifier.
