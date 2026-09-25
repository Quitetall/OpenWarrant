# Domain docs

This repository uses one shared domain context across its Rust workspace.

## Before exploring

1. Read the root `CONTEXT.md` and use its vocabulary.
2. Read relevant authored ADRs under `docs/adr/atoms/`.
3. For governing requirements, read the relevant sections of
   `docs/sas/generated/NORMATIVE.md`. For repository changes, read
   `CONTRIBUTING.md`.

## Layout and changes

`CONTEXT.md` is the shared glossary. `docs/adr/atoms/` holds the authored
first-class ADRs. Follow `AGENTS.md` and the existing registry and lifecycle
when proposing a normative decision. Edit authored sources and compile their
projections through `war`; generated documents are read-only inputs.

Use the glossary's distinctions between a Warrant, SAS, Stage, Gate, Receipt,
Verification, and Resolution in discussions, proposals, and code. Identify a
missing term explicitly before extending the glossary. Check resolved-file
pins before edits, following `AGENTS.md`.

Surface conflicts with an existing ADR by its actual identifier and explain
the conflict. A proposed change remains a proposal until the repository's
required human act establishes it.
