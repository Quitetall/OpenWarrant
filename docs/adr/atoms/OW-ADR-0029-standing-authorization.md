---
schema: oh.war/atom/v1
adr_uuid: 01a0d28c-78b1-7a3c-895f-4750ab98eb27
local_alias: OW-ADR-0029
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0d289-1e89-7990-8b2c-5a43577b5ce5"
---

# ADR OW-0029: Standing authorization — one signature authorizes a bounded class of routine work

## Status

Proposed by the performer under OW-WAR-0142. **Not decided.** The owner
chooses A, B or C by answering OW-WAR-0142 Q-001. Until then this ADR
records options and a recommendation, and it governs nothing. If the answer
is A, the ADR is adopted when the owner accepts the SAS revision it
carries (SAS 1.2.0, additive; see "What A changes in the SAS"). If the
answer is B or C, the Decision section is rewritten to that answer and
the options not taken stay below as the record of the choice.

## Context

On 2026-09-24 the owner compared `war` with a Jira ticket: *"nobody's gonna
run that command to just do work when they could just prompt or hit a jira
ticket ... Reduced friction? Compare us to jira tickets."*

A Jira ticket costs about a minute and no signature, and nothing waits on
anyone before work starts. A Warrant costs two human acts:

- an **authorization** before work (§28.4), two dialogs: the response and
  its attestation;
- a **resolution** after work (§56), two more dialogs.

Batching (OW-WAR-0072, `docs/SIGNING.md`) makes N acts cost two dialogs.
It does not remove the wait. A batch holds one role, so authorizations
and resolutions are two batches. Every change still waits for the
authorize batch before it can start. The wait makes a Warrant slower than a
ticket, whatever the batch saves in dialogs.

Most of this repository's changes are routine: a bounded edit to named
code paths, checked by the battery and by the blind verifier (OW-WAR-0117),
at `basic` assurance. For these, the authorization adds no information the
human did not already give for the previous twenty such changes.

These texts constrain the answer:

- **§27.2.** An agent SHALL NOT authorize its own proposed WAR, resolve its
  own delivery, accept residual risk, or change a gate definition it is
  judged by. `docs/authority/roles.toml` refuses these to an agent by
  *kind*, whatever roles it holds.
- **§28.4 and §37.5.** An authorization records the contract digest,
  authorizer, acting role, meaning, effective time, policy basis and the
  declared deliverable paths *"as the authorizer saw it"*.
- **§28.7.** An authorized contract is never patched.
- **§30.** The autonomy envelope. §30.2 is the precedent: *"A narrow policy
  may pre-authorize"* certain revisions of an authorized contract. *"The
  governing policy or ADR made the normative decision. The instance still
  creates an immutable revision and audit event."* §30.3 lists what always
  needs a manual revision, including scope expansion, security boundary
  change, new production dependency, gate weakening and accepted residual
  risk. §30.4: an executor SHALL NOT improvise normative semantics.
- **§27.3.** A policy service MAY resolve a basic mechanical WAR. The owner
  has ruled that out for this work: *a resolution is never automatic*.
- **OW-ADR-0021.** A path belongs to the latest authorized Warrant that
  declares it. Ownership is granted by the signed deliverable set.

## Options

Costs are for one routine change. A "round" is however many changes the
owner reviews at one sitting (N). A dialog is one ssh-agent confirmation.

| | before work | after work | dialogs per round | waits before work? | reads per change |
|---|---|---|---|---|---|
| Jira ticket | none | none (no verification) | 0 | no | 1 |
| today, one act at a time | authorize (2) | resolve (2) | 4N | yes | 2 |
| **B** batching + presets only | one authorize batch (2) | one resolve batch (2) | 4 | **yes** | 2 |
| **A** standing authorization | nothing | one resolve batch (2) | 2, plus 2 per class signed | no | 1 |
| **C** SAS revision of §27.2 | nothing | one resolve batch (2) | 2 | no | 1 |

### A — a standing authorization record, checked for every Warrant against its class

The owner signs one `oh.war/standing-authorization/v1` record. It names a
**class** of work, and its terms are closed. A term the record does not
state is refused, never guessed (§30.4):

- **paths.** Globs a covered Warrant may declare as deliverables.
- **profile.** `delivery` only.
- **assurance.** At most `basic`.
- **gates.** Each obligation must cite one of the class's gates. The class
  must include `gate://ops.conformance.plants@1.0.0` and
  `gate://software.repo.war-check@1.0.0`, and a blind verification is
  required before resolution.
- **budget.** A ceiling on each stage's `budget_tokens` and
  `wall_time_seconds`, and on the number of stages and deliverables in one
  Warrant.
- **bounds on the class itself.** An expiry (`expires_at`) and a count (at
  most `max_warrants` covered Warrants), after which it covers nothing.
- **meaning.** The owner's own words for what the signature grants.

The class record carries no term for residual risk, for an ADR atom or for
resolution. So a covered Warrant cannot carry an
`accepted_residual_risk` assumption or an `adr` atom, and it cannot be
resolved by anything but a human act.

**The paths no class may cover.** This set is non-waivable. It is compiled
into the tool, not read from any file the class or an agent can edit:

- the authority files: `docs/authority/**`, `openwarrant.toml`,
  `docs/sas/**`, `docs/roadmap/**` and `docs/adr/**`;
- the gates and their fixtures: `docs/gates/**` and `conformance/**`;
- the guards: `.claude/hooks/**` and `.claude-plugin/**`;
- dependency manifests: `Cargo.toml` and `Cargo.lock` at any depth. A new
  production dependency is §30.3;
- the code that decides authority: `standing.rs`, `authorize.rs`,
  `sign.rs`, `batch_cmd.rs`, `authority_check.rs`, `attest.rs`,
  `verify.rs`, `resolve.rs`, `resolution_cmd.rs` and `ownership.rs`. The
  class cannot cover the code that checks it. Editing the coverage check
  would widen every class, so it is a security-boundary change (§30.3);
- every Warrant's `authorization*.toml`, `deliverables.toml` of a resolved
  Warrant, and `generated/**`.

`check.rs` stays coverable, although it calls the coverage check.
Removing that call fails `conformance/plants.d/69-standing.sh`. No class
can touch that plant, and the battery runs before any resolution.

A class whose globs name, or match, any of these is refused when it is
proposed and again when it is signed.

**How a Warrant is covered.** An agent writes a Warrant with
`[standing] ref = "standing://<id>@<revision>"`. `war standing apply
<alias>` then runs the coverage check. The check is deterministic, needs
no model and no network, and compares the compiled contract with the
class, term by term. The result is one of two:

- **Covered.** It writes the same `authorization.toml` a signed response
  writes: an immutable authorized Contract Revision for this Warrant's exact
  contract digest (§28.4, §28.7). The record carries:
  - the human who signed the class as `authorizer`;
  - `policy_basis = "standing://<id>@<revision>"`;
  - the class's digest;
  - the Warrant's own deliverable set.

  One audit event is journaled, as §30.2 asks of the instance.
- **Refused.** The refusal names the term the Warrant breaks, and nothing
  is written.

`war check` does not trust the record. It re-derives the record on every
run:

- the class signature verifies against `allowed_signers` as a human's;
- the class was unexpired and under its count when the record's
  effective time was stamped;
- the Warrant, as authorized, is inside the class.

**What the agent can and cannot do.** The agent drafts, performs, runs
the battery and asks the blind verifier. It cannot sign the class:
`roles.toml` refuses an agent by kind. It cannot widen the class: the
class is a signed record, and changing one byte breaks its signature. A
wider class is a new revision with a new human signature, and it covers
only Warrants applied after it. §31 rules out reinterpreting earlier ones.
It cannot resolve: resolution stays the existing human act, batched.

**Cost per routine change:**

- before work: none;
- after work: one line in a resolve batch, 2 dialogs per round;
- once per class: the class signature, 2 dialogs.

**What A changes in the SAS.** A does not change who may perform the acts
§27.2 reserves. The authorizer of every covered Warrant is the human who
signed its class. The tension is with two other sentences:

- §28.4 and §37.5 say the authorization records the contract digest and
  deliverable set *as the authorizer saw it*. Under A, the authorizer saw
  the class, not the instance.

So A carries an additive revision, SAS 1.2.0:

- a new §28.8 *Standing authorization*: a human authorization may cover a
  closed class of contracts. Each instance still records an immutable
  authorized revision with its own digest and set, and names the class
  it was checked against;
- one clause in §37.5: under a standing authorization, the set is what
  the class bounded;
- one §106 row: *A standing authorization covers only contracts inside
  its signed class, and never an authority path.*

No row is removed or renumbered. The revision is architecture-changing
under §101.3, and this ADR is the record it carries. Accepting it is the
owner's separate act.

### B — no standing authorization: batching and one-click signing only

B builds nothing new. The routine flow is written down:

- a preset authorize meaning (preset 4 exists);
- `war sign --batch` over the round's authorizations;
- work;
- the blind verifier;
- a batch over the round's resolutions (preset 5 exists).

**Cost per routine change:** 4 dialogs per round, spread over N changes.
Each change is read twice. **Every change waits for the owner before it
can start.** B keeps the SAS as it is and adds no attack surface. It
loses the Jira comparison on latency, which is the owner's complaint.

### C — a SAS revision redefining which acts need a human

Change §27.2 itself. Routine work would need no authorization at all, or
a §27.3-style policy service would authorize it the way it may now resolve.
The class would be declared in policy (`openwarrant.toml`).

**Cost per routine change:** the same as A. It could fall to zero only if
resolution were automated too, and the owner has ruled that out.

The cost to the standard is larger. Today every authorized Warrant has a
human authorizer, and C removes that invariant. The class would move into
a file an agent can edit, unless it too is signed. Once the class is
signed, C has the same shape as A, with a weaker sentence in §27.2 as well.

## Recommendation: A

A gives the per-change cost C gives. It keeps §27.2's list and its meaning:
a human signed, and the record names who. It builds on §30.2's pattern,
where a policy decides in advance and each instance still gets an
immutable record and an audit event, and extends that pattern from
revisions to new instances.

The SAS change A needs is additive and sits beside §28.4. It does not
touch §27.2. B is the safe fallback. It keeps the wait before work, which
is the friction the owner named.

## Decision

*Pending Q-001.* If A: the mechanism above. The terms of a class,
including the never-coverable set, the coverage check and its refusals, are
fixed by this ADR. A later change to any of them is a new ADR.

## Consequences (if A)

- A routine change starts as soon as the agent can write its Warrant. The
  owner's work is one line in a resolution batch, after the battery and
  the blind verifier have run.
- Revocation is one human act (`war sign standing:<id> --revoke`). After
  it, no new Warrant is covered. Warrants already authorized keep their
  authorization, because §31 does not reinterpret prior execution. Their
  resolutions still wait for the owner.
- Ownership under OW-ADR-0021 is unchanged, with one exception: a
  standing-authorized Warrant may not declare a path whose current owner is
  an authorized, unresolved Warrant. Otherwise routine work could silently
  take a file away from work in flight. Draft: refuse. Q-003 asks the
  owner.
- A class only narrows what the owner could already sign one Warrant at a
  time. The residual risk is a class drawn too wide, such as `src/**`. The
  expiry and the count bound how long a wide class lasts. Only the owner
  can judge how wide is too wide, and the signing screen prints every glob
  and what it matches today.
