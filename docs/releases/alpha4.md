# OpenWarrant 1.0.0-alpha.4 — integration and conformance preview

This alpha updates the planning and tracking CLI from alpha.3. Ordinary work
still needs no signature. Use `war init --vibe`, `war create`, `war claim` and
`war done`; human signing remains a separate assurance action.

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
