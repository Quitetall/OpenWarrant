---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f2a-8e39-69730f255e33
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The WAR protocol exists as a specification and as nothing else. There is no
repository, no workspace, and no executable that can read a manifest, so every
statement in the SAS is currently unfalsifiable by running anything.

The decision-record system it replaces is measurably failing in the meantime: of
94 declared gates in the LamQuant corpus (measured once at LamQuant `5369da81`, 2026-08-17, and historical — see the README; LamQuant is being repaired against these findings), 43 pass and 23 invoke a tool, script,
or crate that is not in the tree. Those are not failing checks; they are
sentences shaped like checks. The SAS was written to make that class of claim
impossible, and it cannot do so from a Markdown file.

## Desired Outcome

A Rust workspace exists, builds, and is governed by one aggregate gate that has
been observed to fail. `war init` initializes a repository end to end. The
boundaries the SAS assigns to other kernels are visible in the crate layout from
the first commit rather than negotiated after the fact.

## Scope

The repository skeleton, the four v0 crates, the pinned toolchain, the license
gate, CI, and `war init`.

## Non-goals

- No manifest parsing, canonical IR, or projections. Those are OW-WAR-0002
  through OW-WAR-0004.
- No RFC 8785 canonicalization. Choosing that implementation binds the wire
  format for every cross-system digest and requires its own implementation ADR.
- No Knowledge Fabric registration, Katana execution, or Liminal integration.
- No agent planner. `openwarrant-agent` carries protocol surface only.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-064` — OpenWarrant does not duplicate the KF, Liminal, Katana, or
  BLUT kernels. Partial: the crate layout establishes the seams; the adapters
  that must respect them do not exist yet.
- `WAR-SAS-RQ-070` — the CLI works file-native and offline for drafts. Partial:
  `war init` is offline and file-native; the rest of the offline surface follows.
