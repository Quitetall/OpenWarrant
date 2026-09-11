---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-78ce-74b8-b6e8-7b8ea92afa04
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

SAS §14 requires every compilation to run against one immutable Workspace Basis,
and states that a compilation "SHALL NOT silently mix independently changing
inputs." The Basis enumerates manifest, atom, ADR, SAS, roadmap, schema pack,
compiler and receipt revisions — all inputs owned by the program itself.

A program whose software depends on another repository has an input the Basis
cannot currently express: the exact revision of that upstream, the toolchain it
is built with, and the interface schema and tolerance profile agreed at the
boundary. Nothing in the corpus records it, so nothing detects when it moves.

The failure is silent by construction. The upstream advances, the downstream
Warrant still compiles, its contract digest does not move, and the record
continues to assert a basis that no longer exists.

## Desired outcome

An external dependency pin is a first-class Compilation Basis input. Pinning
data lives in a digested file the Warrant owns, under its own digest domain, so
that changing a pin moves the contract digest and the Warrant reads as amended
rather than as unchanged.

Malformed pins are refused before they are recorded: a revision that is not
exactly 40 lowercase hex characters, or a toolchain given as a channel name or
a range rather than an exact version, is a refusal with a named rule.

## Bound and non-goals

Scope: a new basis input file, its schema, its digest domain, the refusals that
guard it, and conformance plants for each refusal.

Out of scope: fetching, updating, or resolving upstream revisions; any network
access; any opinion about which revision is correct; submodule mechanics; and
migration of any consuming program onto this facility. This Warrant adds an
expressible input, not a dependency manager.
