# OpenWarrant 1.0.0-alpha.5 — Format 1.0 draft 3 and a short first read for agents

This unverified alpha updates the planning and tracking CLI from alpha.4. Ordinary work
still needs no signature: `war init`, `war create`, `war claim`, `war done`.
Human signing remains a separate assurance action.

What is new for a user:

- **OpenWarrant Format 1.0, draft 3 (revision 2).** Checks and KPIs that run on
  `war done`, on a commit, on changed paths, on CI or on demand
  (`war check run`); tick levels; work tracking with one `next` order and
  journal v2. The conformance corpus has 206 cases and both `war` and an
  independent Python reader pass all of them. OW-ADR-0032 (Format 1.0 is the
  standard) is accepted; the edition is not yet frozen and may still change.
- **A short first read for agents.** `war view prime` is a briefing however
  much a repository has written (34 KB to 12 KB here), and AGENTS.md now starts
  an agent there and at `war context <id>`, not at the multi-megabyte
  `docs/generated/CURRENT.md`.
- **A repository with no Warrant is checked.** `war check` reads the Format 1.0
  documents of a repository set up with `war init` alone; before, a misspelled
  key there checked clean.
- **Working-form Warrants in every view**: overview, JSON snapshot, offline
  HTML and the local progress app, with tick evidence shown beside completion.
- **Faster `war status` and `war next`**: about half the CPU of alpha.4 on a
  large corpus.

Also:

- A replaced Warrant is no longer shown as current or offered by `war next`.
- The release bundle keeps the full committed source in a bounded
  `candidate-bundle/v2` capsule; the installer still reads alpha.4's form.
- Retained BLUT and Katana replay observations and the OW-WAR-0100 delivery
  record. These are records of bounded prototype runs; they qualify nothing.

This is an unverified alpha preview, not Stable 1.0. Legacy reconciliation,
independent qualification, secure human acceptance and the adoption study
remain incomplete. No deployment permission is granted.

The release workflow must validate the exact candidate archives and native
prototype install receipts for Linux x86_64 and macOS arm64 before publishing.
An existing alpha.4 installation remains unchanged until explicitly upgraded.
