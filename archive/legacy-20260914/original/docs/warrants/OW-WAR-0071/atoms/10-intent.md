---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32d7-7473-b6e1-fbef061a1e5f
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Signing is the friction, and the friction is per-act. Clearing a day's work
means twenty-four `ssh-add -c` dialogs: each act drafts its own response
document, each document is signed on its own, and the human's twenty-four
confirmations differ from one another only in which alias they name. The
owner's words on 2026-09-12: "This is a lot of work for basically something
supposed to be nearly instant" and "Nobody wants to open up another terminal
and sign 10 warrants and have to write 20 sentences."

Two remedies exist and only one is honest. Dropping `-c` removes the dialogs
by removing the control `docs/THREAT_MODEL.md` entry 1 names — the key then
signs whatever asks it, including an agent that reached `SSH_AUTH_SOCK`. The
other is to make **one** signature cover a list the human read: the human act
stays a human act, the dialog count falls to one, and what they signed is a
document naming every act it authorizes.

The SAS as accepted at 1.0.0 does not admit that. §27.2 names the human-only
acts one at a time; §34.4, §38 and §56.1 each describe a signature bound to a
single record; nothing describes a signature whose subject is a list. The
revision this Warrant governs is what makes the batch act sayable, and the
same revision admits a second thing the 1.0.0 text does not: a terminal
dashboard, which is a *rendering* of the corpus with no authority of its own
and must be written down as such before one is built.

## Desired Outcome

SAS 1.1.0, accepted by the owner's signature, in which: a batch act is
defined as one signature over a canonical list of pending acts, with the
rule that a batch may only contain acts the signer could have performed
individually and that a moved digest invalidates the batch rather than
silently re-scoping it; §27.2 keeps every act human-only; and a rendering
(the terminal dashboard, the Pages projection) is defined as a view that
issues commands and never holds authority. `OW-ADR-0019` carries the
decision §101.3 requires for an architecture-changing revision.

## Non-Goals

- Implementing the batch act (OW-WAR-0072) or the dashboard (OW-WAR-0073).
- Relaxing `-c`: this revision exists so the owner need not.
- Multi-signer batches. One signer, as 1.0's audience decision says.
