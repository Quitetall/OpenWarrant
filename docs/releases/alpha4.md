# OpenWarrant 1.0.0-alpha.4 — integration and conformance preview

This alpha updates the planning and tracking CLI from alpha.3. Ordinary work
still needs no signature: `war init`, `war create`, `war claim`, `war done`.
Human signing remains a separate assurance action.

What is new for a user:

- **OpenWarrant Format 1.0 (draft 2).** A Warrant can be one Markdown file
  (`docs/warrants/<alias>.md`), read by `war` beside the older directory
  form. The format is specified in `docs/format/FORMAT-1.0.md` with JSON
  schemas and a conformance corpus; it is a draft and will change before
  1.0.0.
- **`war context <target>`** gives an agent exactly the records a piece of
  work needs, within a byte budget, the same bytes every time.
  `docs/generated/llms.txt` indexes every record with its state.
- **`war init` defaults to the `vibe` preset**, and the guided setup
  (`war init --guided`) no longer loops at the first Warrant's sign step.
- **Installs:** cargo-binstall metadata and a Homebrew formula asset with
  each release.
- **Smaller download:** the bundle's source archive leaves out the
  repository's own evidence (about 44 MB of source instead of 242 MB).

Also:

- Update rustls to 0.23.46 to address RUSTSEC-2026-0346 through
  RUSTSEC-2026-0349. No advisory exceptions were added.
- Retain a repeatable Katana CLI capture fixture and its usage notes under
  `conformance/fixtures/runtime-capture/`. This exercises a real CLI against a
  scripted local endpoint; it does not establish paid-model cost accounting,
  execution confinement or operator qualification.
- Keep native Phase 1 observations and source-inventory diagnostics distinct
  from acceptance. Passing checks do not sign or resolve legacy Warrants.
- Give the release gate the same finite 75-minute job limit as CI. Its checks,
  planted refusals, individual deadlines and publication safeguards remain.

This is an unverified alpha preview, not Stable 1.0. Legacy reconciliation,
independent qualification, secure human acceptance and the three-developer
usability study remain incomplete. No deployment permission is granted.

The release workflow must validate the exact candidate archives and native
prototype install receipts for Linux x86_64 and macOS arm64 before publishing.
An existing alpha.3 installation remains unchanged until explicitly upgraded.
