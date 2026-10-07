---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e89-7990-8b2c-5a43577b5ce5
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- **§27.2.** An agent SHALL NOT authorize its own proposed WAR, resolve its
  own delivery, accept residual risk, or change a gate definition it is
  judged by. `docs/authority/roles.toml` refuses these to an agent by
  kind.
- **§27.3.** A policy service MAY resolve a basic mechanical WAR. Out of
  scope here by the owner's rule: no resolution is automatic.
- **§28.4.** Authorization records the contract digest, authorizer, acting
  role, meaning, effective time, policy basis, Compilation Basis and the
  declared paths as the authorizer saw them.
- **§28.7.** An authorized contract is never patched.
- **§30.1–§30.4.** The autonomy envelope. §30.2 is the precedent: *"A
  narrow policy may pre-authorize … The governing policy or ADR made the
  normative decision. The instance still creates an immutable revision
  and audit event."* §30.3 is the band no class may reach. §30.4:
  `on_ambiguity: block_and_propose`.
- **§31.** An amendment SHALL NOT retroactively reinterpret prior
  execution. A wider class does not re-cover earlier Warrants.
- **§37.5.** An authorization records the declared path set *"as the
  authorizer saw it"*.
- **§38.1.** Completion is decomposed into bounded obligations.
- **§46.** Independent verification; RQ-053.
- **§56.1.** Resolution verifies the exact authorized revision and that
  the resolver holds the role.
- **OW-ADR-0021.** Ownership is granted by the signed deliverable set, and
  the latest authorized declarer owns the path.
- **OW-ADR-0029.** This Warrant's decision (ordinal 30).
- **OW-WAR-0072.** The batch act. **OW-WAR-0117.** The blind verifier.
- **Code read.** `crates/openwarrant-core/src/autonomy.rs` (the three §30
  bands; `classify` returns `None` rather than guessing).
  `crates/openwarrant-cli/src/authorize.rs` (request and response, the
  refusals, `authorizes_current_contract`). `sign.rs` and `batch_cmd.rs`
  (one signer and one role per batch; a resolution batches when §38.6
  permits `satisfied`). `docs/SIGNING.md`.

## Where the tension is

§27.2 forbids an **agent** authorizing. Under A the authorizer of record is
the human who signed the class. The agent triggers a deterministic check,
and the check either writes the record or refuses.

The sentences under strain are §28.4 and §37.5. They say the record holds
what the authorizer *saw*, and under A the authorizer saw the class, not
the instance. The draft's reading is that A needs an additive SAS revision
(1.2.0: a new §28.8, one clause in §37.5, one new §106 row) and no change
to §27.2. C is the option that changes §27.2. The owner decides whether
that reading is right (Q-001).

## Assumptions

- A-001: every covered Warrant can be judged against the class from its
  compiled contract alone: declared paths, profile, assurance level, gate
  refs, stage budgets, the ADR atom, residual-risk assumptions. No model
  and no network are needed. Confidence: high. Each of these is already
  a field `war check` reads.
- A-002: the owner's routine work fits a class. It is `basic`, delivery,
  and confined to code paths outside the never-coverable set.
  Confidence: medium. Many recent Warrants touch `sign.rs` or `check.rs`.
  `sign.rs` is never coverable, while `check.rs` is coverable, so some
  routine work stays per-Warrant.
- A-003: a class's globs can be shown with what they match today on the
  signing screen, so the signer sees the class's reach. Confidence: high.

## Unknowns

- U-001 (blocking, STAGE-001): whether the owner chooses A, B or C (Q-001).
- U-002 (blocking, STAGE-003, only under A): the default bounds of a class,
  its expiry and its count cap (Q-002).
- U-003 (blocking, STAGE-003, only under A): whether a covered Warrant may
  take a path from an authorized, unresolved owner (Q-003).

## Constraints

- The performer drafts and never signs. The class, its revocation, SAS
  1.2.0's acceptance and every resolution are the owner's acts.
- Pinned files in `deliverables.toml` are edited only after this Warrant
  is authorized (OW-ADR-0021).
- No existing record schema changes. The class is a new schema, and a
  covered Warrant's `authorization.toml` is the existing
  `oh.war/authorization/v1` with `policy_basis` set.

## Residual risks

- R-001: a class drawn too wide, such as `crates/**`. The tool bounds it
  (the never-coverable set, the expiry, the count) and shows it (the
  globs and their matches on the signing screen). How wide is too wide is
  the owner's judgment, and no rule settles it.
- R-002: routine work under A gets less of the owner's attention before it
  starts. That is the intent. The battery and the blind verifier run
  before the owner reads, and the owner still reads each change once,
  at acceptance.
