---
schema: oh.war/atom/v1
adr_uuid: 01a0f547-44a2-7d27-9d77-43d36f651cd1
local_alias: OW-ADR-0030
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
---

# ADR OW-0030: Bind a verdict to the subject that was reviewed

## Status

Proposed implementation decision for [ticket t-b9610](../../tickets/t-b9610/atoms/10-intent.md). Not adopted. The implementation branch is experimental and unfinished; this document grants no authority and changes no signed historical record.

## Evidence and problem

A public CLI reproduction on a disposable fixture ingested an independent response, changed the requirement, and still reported verification current. The old response could be ingested again. Historical observations cannot establish a different current subject.

## Proposed representation

Add `reviewed_subject` to verification requests. A returned response can echo this exact object. Initially it contains the existing compiled contract digest and a sorted map of repository-relative delivered paths to the ordinary SHA-256 of their actual bytes. A missing artifact is recorded as missing; this does not satisfy the separate existence gate. Malformed declarations cannot produce an empty snapshot.

Keep the existing core verification record bytes. The existing verification-recorded journal event binds their exact file digest, actor, obligation and reviewed subject. Current qualification requires this event to match the current subject. A legacy response without the new field can be retained, but supplies no current subject binding: qualification is UNKNOWN. Never infer binding from timestamps or alter old dispositions.

This is an additive wire representation in the experimental CLI and therefore a documented decision, not replacement canonicalization. Contract and verification-bundle digests retain their existing canonicalizers and domains. The full portable review must also bind the existing canonical verification-bundle identity and the relevant source/evidence facts; that portion is not yet implemented. Do not present the initial contract/artifact map as complete context binding.

The configured wrapper reads one private copy of the supplied bundle and echoes its request subject. It must not recapture a newer subject after review, ask a model to invent the binding, or infer it when a legacy request has none.

Before writing any verdict, ingestion rejects a response about a stale subject or undeclared obligation. Current preparation, resolution, progress and assurance-mark evaluation use the same qualification rule. Git-candidate acceptance must use the subject at that exact candidate, not whatever is in the current worktree; that path remains unfinished.

## Adoption and remaining work

This proposal does not weaken independent or human gates. Required fixture/context coverage, canonical bundle identity, candidate acceptance, path and race controls, conformance compatibility, and the full gate remain to be proved. Human acceptance is separate from this draft and its local tests.
