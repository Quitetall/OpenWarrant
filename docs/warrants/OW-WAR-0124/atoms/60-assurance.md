---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f0f-7e31-bed2-efca03dd781e
role: assurance
jurisdiction: authored
order: 60
classification: internal
---


# Assurance

## Acceptance Obligations

### OBL-001 — init records the baseline of a repository with history, and refuses one that is not a commit
- **scope:** `war init --program` on scratch Git repositories built by
  `59-adoption.sh`. No claim about repositories with submodules or
  multiple roots.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - after three commits, `openwarrant.toml` has `[adoption] baseline`
    equal to `git rev-parse HEAD`, and `war check` exits 0;
  - `--baseline 0000000` exits non-zero and no `openwarrant.toml` exists
    afterwards;
  - a repository with no commits gets no `[adoption]` table, and
    `99-init.sh` and `75-init-program.sh` pass unmodified.

### OBL-002 — untracked work counts from the baseline, in the repository's namespace
- **scope:** `war telemetry` on the same scratch repository.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - 0 candidates right after init;
  - 1 after one uncited commit;
  - still 1 after a commit citing `ACME-WAR-0001`;
  - 2 after a commit citing only `OW-WAR-0001`;
  - the report names the baseline it used.

### OBL-003 — nothing before the baseline is claimed or owned
- **scope:** the scaffolded adopt Warrant and `war pins` on the scratch
  repository.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the adopt Warrant's `20-basis.md` contains the baseline id and the
    sentence that nothing before it is claimed, owned or verified;
  - `war pins` lists none of the three pre-baseline files;
  - `war resolve --dry-run` on the adopt Warrant reports no established
    obligation.

### OBL-004 — existing ADRs are pointed at and never imported
- **scope:** a scratch repository with `docs/adr/0001-x.md`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war init` prints one line containing
  `war migrate --corpus docs/adr --commit <baseline>`; no file under
  `artifacts/` exists afterwards; without the directory the line is
  absent.

### OBL-005 — this repository's telemetry does not move
- **scope:** this repository, which has no `[adoption]` table.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war telemetry --commit <the committed baseline's commit>
  --verify` against `artifacts/telemetry-baseline.json` still passes.

### OBL-006 — the guided setup asks for the baseline and changes no authority step
- **scope:** `init::guided` unit tests with canned answers, and a grep of
  the diff to `guided.rs`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - with a history, the step after Program is Baseline, and confirming
    yields exactly one `Effect` writing `[adoption]`;
  - with no history, Baseline is skipped;
  - the existing tests `an_existing_authority_file_is_never_written_again`
    and `the_agent_is_a_performer_and_nothing_else` pass unmodified;
  - no line of the diff touches `render_roles`, `render_allowed_signers`
    or the Signer and KeyLoaded arms.

### OBL-007 — init makes the directory a git repository, and never nests one
- **scope:** `war init` and `war init --program` on scratch directories:
  one outside any work tree, one inside an existing repository, and one
  with `git` removed from PATH.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - outside a work tree: a `.git` exists afterwards, init's output names
    it, and `war check` on the scaffold is ready (no UNKNOWN
    `identity.changed`);
  - inside an existing repository (a subdirectory of one): no `.git` is
    created in the subdirectory and the enclosing repository's HEAD is
    unchanged — the refusal to nest;
  - with no `git` on PATH: init exits 0, writes the scaffold, and its
    output says history cannot be checked; `war check` reports UNKNOWN
    `identity.changed`, never a pass.

## Gate Adequacy

Required at `controlled`, because the work edits the module that writes
the authority files. The load-bearing obligations are OBL-003, since a
brownfield adoption that implies its history was governed is the
fabrication §96.3 forbids, and OBL-006, which shows the authority path
was not moved to make room for the new step.

**Adversarial question:** can an artifact pass every declared gate while
hiding work from untracked-work detection? Yes: a baseline edited by hand
to a later commit after adoption. No plant here refuses it, because this
Warrant adds no rule on moving the baseline (Basis, U-001). The telemetry
report names the baseline it used, so the move is visible where the count
is read, and not prevented.

- **outcome:** gap_accepted

**Limitation:** the review was done at drafting, on the design. No attack
has been executed yet; the plants in `59-adoption.sh` are the planned
negative controls.
