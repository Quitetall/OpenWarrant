---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f0f-7e31-bed2-efca03dd781e
role: intent
jurisdiction: authored
order: 10
classification: internal
---


# Intent

## Problem

`war init` assumes a new repository. A repository with years of commits
and no Warrants gets the same scaffold, and two things then go wrong.

- **Every past commit is untracked work.** `war telemetry` lists every
  commit in `git log` whose subject cites no Warrant
  (`untracked_candidates` in `telemetry.rs`). Nothing marks where
  OpenWarrant began, so a repository adopted at its 3,000th commit starts
  with 3,000 candidates. §95 calls these a governance signal. A signal
  that is always on is noise.
- **Other namespaces never match.** The same function recognises a
  Warrant by the literal prefix `OW-WAR-`. In a repository whose namespace
  is `ACME`, a commit citing `ACME-WAR-0001` is still a candidate. Every
  adopter other than this repository is affected.
- **Nothing says what the history is.** The scaffold's first Warrant,
  "Adopt OpenWarrant in <program>", does not name the commit it starts
  from or say that earlier work is neither claimed nor verified. The
  product spec requires legacy work to be imported "in their actual
  states … Do not fabricate retrospective acceptance or provenance".
- **Existing decision records are not found.** `war migrate` imports an
  ADR corpus (§96), but `war init` does not look for one or mention it.

## Desired Outcome

A repository with history adopts OpenWarrant through the same `war init`,
and the result says truthfully where governed work begins.

- **An adoption baseline.** When the repository has commits, `war init`
  records `[adoption] baseline = "<full commit id>"` in
  `openwarrant.toml`: HEAD by default, or `--baseline <commit>`. At a
  terminal, the guided setup asks, showing the commit count and the
  commit it proposes.
- **What the baseline means, in writing.** The first Warrant's Basis names
  the baseline and states that nothing before it is claimed, owned or
  verified by any Warrant. `war pins` lists no path from before it.
- **Untracked work counts from the baseline.** `war telemetry` reads
  `baseline..HEAD` when a baseline is recorded, and recognises the
  repository's own namespace. Without a baseline it reads all history, as
  today.
- **Existing ADRs are pointed at, not imported.** When `war init` finds a
  directory of `NNNN-*.md` files under `docs/adr`, `doc/adr` or `adr`, it
  prints the `war migrate --corpus <dir> --commit <baseline>` command that
  would import them. It runs nothing.
- **`docs/ADOPTING.md`** walks the path: init, the baseline, the SAS, the
  first Warrant, existing ADRs, and what the owner signs.
- **A new repository is unchanged.** With no commits, `war init` writes no
  `[adoption]` table and its output is byte-for-byte what it is today.

## Non-goals

- Retroactive Warrants for past work, or any record claiming that past
  work was authorized or verified.
- Importing ADRs automatically, or any change to `war migrate`.
- Migrating existing atom/parent documentation (§97). That needs a
  repository that already has atoms, and a later Warrant.
- Changing any authority step of the guided setup: who signs, the key
  question, `roles.toml` and `allowed_signers` stay as OW-WAR-0112 built
  them.
- Measuring the setup-time target (≤10 minutes). That is
  `roadmap://OW-PHASE-0/friction-baseline`.
