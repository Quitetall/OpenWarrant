---
schema: oh.war/atom/v1
warrant_uuid: 01a0ca4a-0c02-7cd3-b49d-786377a1aa06
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — ownership is granted only by a signature over the declared set
- **scope:** `authorize.rs`, `sign.rs`, `authorization.toml`, the attestation.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** on a scratch program, a response whose `deliverable_set_digest` differs from the manifest is refused as `authorize.stale-deliverables` before any signature is verified and writes nothing; a path added to `deliverables.toml` after signing is reported `deliverable.undeclared-at-authorization` and is not owned; a path edited into `owned` in `authorization.toml` is `attest.subject-drift`; an authorization whose effective time precedes an existing owner of the same path is refused `authorize.time-before-owner`. `war sign --show` on a pending authorization prints "Grants ownership of:" followed by every declared path.

### OBL-002 — a historical pin passes, an undeclared edit still drifts, and history stays readable
- **scope:** `check.rs`'s drift decision, `pins.rs`, `correct.rs`, `resolution_cmd.rs`, `guard-pins.sh`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** on this corpus after STAGE-002, `war check OW-WAR-0005` reports `deliverable.superseded-by` naming OW-WAR-0112 for `check.rs`, exit 0 on that row; on a scratch program, an edit to a resolved deliverable no authorized Warrant declares is `deliverable.digest-drift` whose message names both remedies; `war correct` against a historical pin refuses `correction.historical`; the hook permits an edit to a historical pin and denies an edit to a current one; `war pins --history` on a path with two owners renders both, oldest first, and a resolution recorded after this Warrant carries a `[locator]` whose `commit_sha` is forty lowercase hex.

### OBL-003 — every non-pass diagnostic carries a remedy, and no automatic remedy signs
- **scope:** `remedy.rs`, `diagnostic.rs`, `output.rs`, `schemas/oh.war/report/v1.json`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a unit test walks the remedy table and every fallback path and asserts no `auto` argv contains `sign`, `authorize`, `resolve`, `correct`, `accept` or `answer`; `war check --json` on a planted drift carries `remedy.kind = "human"` and an argv beginning `war sign`; on a planted stale projection carries `remedy.kind = "auto"` and `war compile`; `war check` ends with a `REMEDIES:` block; `89-schemas.sh` passes with `SCHEMA_PACK_VERSION` unchanged.

### OBL-004 — the guided init writes the authority files only from a terminal, once, and never for an agent
- **scope:** `init/guided.rs`, `docs/authority/`, the four documents.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war init --program X --namespace Y </dev/null` leaves no `roles.toml` and no `allowed_signers` and its output is byte-identical to the release before this Warrant; a second run against existing authority files refuses to touch them; the machine driven with canned answers grants an `actor_kind = "agent"` entry `performer` and nothing else; the header the tool writes names the date, the typed name and the `-c` answer; the four documents state the rule in the same commit.

### OBL-005 — the app holds no key and is a rendering
- **scope:** `crates/openwarrant-cli/src/tui/`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** a source grep of `tui/` finds no `ssh-keygen`, no `SSH_AUTH_SOCK`, no `sign::run`; every act goes through `shell.rs` as `current_exe --root <root> --json …`; the queue pane's rows equal `war sign --list`; `war tui --panic-after-setup` leaves the terminal restored (the alternate-screen leave sequence is in the captured output); the confinement test finds `ratatui` and `crossterm` nowhere outside `tui/`; `cargo tree` shows no executor entered the graph.

### OBL-006 — `war` with no arguments opens the app at a terminal and refuses everywhere else by name
- **scope:** `lib.rs`'s entry, `main.rs`'s `NOT_YET` list.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war </dev/null` exits 2 with clap's usage and no escape code; `war --json` exits 2 naming `war console --json` and `war status --json`; `war tui` from a pipe is `tui.no-tty`, exit 2; the `sdk` argv scan is untouched and `war --json sdk --request -` still answers with an envelope; every embedded document renders in the Help pane without panic.

### OBL-007 — the re-pin writes the amendment and the next signature is the human's
- **scope:** `sas_repin.rs`, `amendments/`, `88-sas-repin.sh`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war sas repin <alias>` on an authorized, unresolved Warrant writes `AM-<next>.yaml` carrying `sas_revision` and `predecessor_sas_revision`, and `war sign --list` then shows that Warrant's next authorization revision; `repin` on a resolved Warrant refuses `sas.repin-resolved`; `--all --dry-run` writes nothing; a manifest implementing a row the latest revision lacks refuses the whole batch by name and writes nothing.

## Gate Adequacy

Required at `controlled`.

**Adversarial question:** does ownership give an agent a way to claim a file
it was never granted? Four attacks, each a plant that must be seen to refuse
before OBL-001 and OBL-002 are claimed: a path appended to
`deliverables.toml` after the human signed; a response carrying a set digest
the drafter invented; a path edited into the `owned` list of a record already
signed; and a draft Warrant, never authorized, that declares a path and edits
it. The first three refuse at ingest or under the attestation; the fourth is
`deliverable.pin-stale` today and stays so — a draft's pin is a note, not a
promise, and the file it names is still governed by whoever last was
authorized for it.

**Second adversarial question:** can the app become an authority by holding
the terminal? Only by holding a key. It shells out for every act, the plant
greps for the seam, and the confirm dialog belongs to a child process the app
cannot answer for.

**Third:** does an automatic remedy ever sign? The closed list is tested, and
the app shows a `human` remedy and stops.

- **outcome:** gate_added

## Residual Risk

- Ownership orders by a locally stamped time; the refusal in OBL-001 covers
  the case seen, not a clock set back between two signings.
- A pseudo-terminal defeats `at_a_terminal()` for the guided init exactly as
  it defeats `war sign`'s plain path. THREAT_MODEL entry 2's residual,
  accepted once more.
- Until OW-WAR-0072 lands, `war sas repin --all` on this corpus asks the
  owner for one dialog per Warrant. The tool prints the count before writing.
- A `human` remedy is only as good as the human reading it before signing —
  the same claim this system makes everywhere.
