---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e9a-7621-b9fe-dc0738889647
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Recording an amended revision rewrites `authorization.toml` in place. The
revision the owner signed before is gone from the tree, and every
attestation over it stops verifying: on 2026-09-24, recording revision 2 of
27 Warrants produced 27 `attest.subject-drift` errors. The performer
restored each prior file from git history by the digest its attestation
names (commit 244f538e), which is what this Warrant makes the tool do.

Responses are already treated this way: a response a later act replaces is
retired beside its path as `<name>.<digest8>.response.toml` (§34.4:
supersede, never erase), and `war attest --verify` finds it there.
Authorizations were not, because until that day no attested authorization
had been amended.

## Desired Outcome

- Before `war authorize` (and `war sign`, and a batch) writes a new
  `authorization.toml`, the existing one is kept beside it as
  `authorization.<digest8>.toml`, `<digest8>` the first eight hex digits of
  its own sha256. Nothing is overwritten that a signature covered.
- `war attest --all --verify` passes across an amendment with no manual
  step.
- A dry run writes nothing, as today.

## Non-goals

- Changing the authorization record's schema, or keeping revision history
  inside one file.
- Retiring other records (`resolution.toml` is written once; corrections
  are already a chain).
- Any change to what an authorization means, or who may sign it.
