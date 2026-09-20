---
schema: oh.war/atom/v1
warrant_uuid: 01a060de-a1c9-77f2-962a-74ca52797a34
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — every workflow action is pinned by commit SHA
- **scope:** supply chain; ADR 0182.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the `workflows` step of `cargo xtask gate` refuses a `uses:`
  not pinned to a 40-hex SHA; unit tests feed it a tag-pinned action and a
  short SHA and see both refused, and feed it the real `pages.yml` and see
  it pass.

### OBL-002 — the deploy uploads the committed bytes and nothing else
- **scope:** §59.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the workflow compares the sha256 of every file in `_site/`
  with the committed file before uploading and exits non-zero on any
  difference; the `workflows` step refuses a `pages.yml` that does not name
  the committed projection or does not compare digests (unit test).

### OBL-003 — the published page is the committed page
- **scope:** §17.5.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** after the first deploy, the sha256 of the bytes served at
  the Pages URL equals the sha256 of the committed `CORPUS_STATUS.html` at
  the deployed commit. Recorded in the basis when it has happened; until
  then this obligation is not established.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can the published page differ from the committed
projection, or can the workflow that publishes it run code nobody pinned?
The attacks: a `uses:` moved to a tag; a `pages.yml` that renders instead
of copying; a `pages.yml` that copies from somewhere else; a matcher that
skips the common `- uses:` form.

**Executed attacks:** ten unit tests in `xtask`:

- `actions/checkout@v7` → refused as not pinned
- `actions/checkout@3d3c42e5` (short) → refused
- a `pages.yml` copying `somewhere/else.html` with no digest comparison →
  two problems, naming the source and the comparison
- the real `pages.yml` → no problems
- a SHA-pinned action → no problems
- a quoted ref, a local `./` action, a `docker://` ref with and without an
  image digest, a short SHA → each handled as its comment says
- a flow mapping `{uses: x@v1}`, a quoted key `"uses":`, and `uses :` with
  a space before the colon → each refused; `reuses:`, `causes:`, a
  comment, and `uses` in a `run:` string → not keys, not refused

Two of these were found by executing rather than by reading. The first
matcher stripped `uses:` and skipped every `- uses:` line, and the step
reported every workflow pinned while checking nothing; the tag-pinned test
failed, which is what it is for. The second matcher recognised only a key
at the start of a line, and external review named three forms GitHub's
parser accepts that it passed vacuously: a flow mapping, a quoted key, and
a spaced colon. The matcher now finds every `uses` key on a line, wherever
it sits, and refuses each of the three.

- **outcome:** counterexample_found, gate_added

## Residual Risk

- `cargo xtask gate` has no Gate Definition under `docs/gates/`, so
  OBL-001 and OBL-002 cite the corpus gate while their evidence is the
  xtask step that CI runs. Registering xtask as a gate is a separate
  Warrant.
- The digest comparison runs inside the workflow. A change to the workflow
  that removes it is caught by the `workflows` step only if the text
  `sha256sum` disappears; a comparison that stays in the text and stops
  being reached is not caught.
- Pages serves the last successful deploy. If a push to `main` fails to
  deploy, the site shows the previous projection with no banner saying so.
