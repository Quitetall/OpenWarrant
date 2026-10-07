---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f55-7171-906f-45d79a408ce3
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

RQ-065 says native systems retain artifact authority. §11 names them: CAD,
datasets, instruments, invoices, payments, QMS records and other
domain-native facts. §13 gives `external` as a Source Holder kind, and §37.1
shows `target_ref` as a URI (`git://...`), not only a path.

OpenWarrant can only declare a deliverable as a file in this repository.
Every read of a deliverable joins `target_ref` to the repository root:
`resolve.rs` (existence and digests, §56.1 requirements 2 and 3),
`check.rs` (drift), `pins.rs` (refresh), `correct.rs` (corrections),
`bundle.rs` (the verification bundle) and `status.rs` (digest state). All
238 deliverables in this corpus are repository files.

So a Warrant whose real output lives in a native system has two options,
and both are wrong:

- copy the artifact into Git and declare the copy. The Warrant then owns
  it under OW-ADR-0021, pins its bytes, and `war correct` can "correct" it.
  That is OpenWarrant taking authority RQ-065 leaves with the native system.
- declare a reference anyway. Every reader then treats it as a missing
  file: `pins.unreadable`, drift, and "does not exist". That reports a
  question nobody could ask here as a failure (Law 15).

## Desired Outcome

A deliverable can name an artifact held by a native system. OpenWarrant
records the reference and never takes the artifact over:

- a native reference is recognized in one place (`deliverable.rs`) and
  must carry its holder: provenance with a non-`git` Source Holder, and a
  content digest when it claims content addressing;
- no command reads its bytes from the repository, rewrites its digest, or
  corrects it. Each says which system holds it;
- where a command needs the bytes and cannot have them, it says so by name.
  A native deliverable never passes a byte check here, and never reports as
  missing either;
- every existing Warrant is unaffected: repository paths parse, pin and
  own exactly as before;
- a proposed ADR records the reference form the owner chose.

## Non-goals

- Fetching anything from a native system, or any network access.
- Establishing a native artifact's digest so a Warrant can resolve on it.
  That needs a receipt nobody produces yet (U-002); this Warrant reports
  it as unestablished.
- Changing §56.1's thirteen booleans to three-valued results.
- `war document review` over a native-held document.
- Anything Knowledge Fabric does with native artifacts, and RQ-060
  (OW-WAR-0126).
