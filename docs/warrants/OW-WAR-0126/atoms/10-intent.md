---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f41-7a13-9d41-14fd18657244
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

RQ-060 says Knowledge Fabric owns authority and lifecycle. The SAS states the
rule in three places:

- §11.2: when a WAR is registered, KF owns its authoritative lifecycle and
  controlled actions. OpenWarrant SHALL NOT own a second institutional
  database.
- §72.4: the CLI SHALL not bypass KF authority.
- §12.4: a WAR may not claim globally authorized or effective state until it
  is registered through KF.

Today every human act on a Warrant is local. `war sign` authorizes,
resolves, corrects and accepts; the record is a signed file in Git
(`authority_check.rs`). The tool has no notion of a registered Warrant, so
nothing would stop a local resolution of a Warrant whose lifecycle KF holds.
That is two authorities for one lifecycle.

The seam half exists. `war kf act` posts a §67 typed action (`kf.rs`,
OW-WAR-0028). KF owns all 32 action names and records a Warrant as an
institutional object (KF ADR 0019). No Warrant is registered yet:
OW-WAR-0029 and OW-WAR-0044 are authorized and undelivered, and there is no
`war register`.

Nobody has written down which acts stay local after registration, which
become KF actions, and what a local act means before registration.

## Desired Outcome

A proposed ADR that the owner can accept or reject, and that a later
implementation Warrant can plant against. It states:

- the records and acts that stay local for every Warrant, registered or
  not (drafting, `war check`, compilation, the journal, gate runs as local
  candidates under OW-ADR-0005);
- for each human act (authorize, amend, resolve, correct, verification
  ingest, the batch act): what it is before registration, and what it
  becomes after — the §67 action it maps to, by the name `seam.rs` gives,
  or "no §67 action exists" where none does;
- the refusal the tool makes when a local act is attempted on a registered
  Warrant, named so a plant can test it;
- how a view labels a local authorization before registration, so it never
  reads as institutional (§12.4);
- how the tool learns a Warrant is registered, by reference to the record
  OW-WAR-0029 delivers, not by a new one.

## Non-goals

- Implementing the refusal or the labels. A follow-on delivery Warrant does
  that once this ADR is accepted and OW-WAR-0029's registration record
  exists.
- Registration, identifier allocation, or federation (OW-WAR-0029,
  OW-WAR-0044).
- Any change to Knowledge Fabric, its actions, or its lifecycle machine.
- RQ-061 to RQ-064 (Liminal, Katana, BLUT) and RQ-065 (OW-WAR-0127).
