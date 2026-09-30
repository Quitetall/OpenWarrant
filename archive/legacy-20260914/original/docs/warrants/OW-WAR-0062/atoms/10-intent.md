---
schema: oh.war/atom/v1
warrant_uuid: 01a064fc-a6f2-7c43-bed9-fa248b771712
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Lock down what each object in the hierarchy IS, and the one rule that was
misread: a SAS and a Warrant are the same class of artifact at two levels of
importance.

## What happened

A user set out to specify their own program and wrote a Warrant "in the style
of the OpenWarrant SAS" instead of writing a SAS for the program. §6 drew the
hierarchy as a diagram and described each level in a sentence, but nowhere
said that a SAS and a Warrant are the same kind of thing at different scope,
nor gave the rule for choosing. A Warrant with no SAS has no requirement ids
to implement, no Objective to discharge and no Release to belong to; the
projection can only file it under `unassigned`.

## What this delivers

- SAS §6.10 (revision 0.1.0-draft.3): a table fixing every level's object —
  what it is, who writes it, what governs it, what reads it — the rule
  itself, the two decisions that follow from it (starting a program → SAS;
  work inside a program → Warrant), and the level-for-level correspondence
  between a Warrant's parts and a SAS's.
- `docs/DEFINITIONS.md`: the same, restated for a reader who has not opened
  the SAS, one paragraph per object.
- The README's status section replaced: it asserted counts of resolved
  Warrants that no record supported. The ladder lives where it is computed.
- Every `war new` manifest starts with the pointer, so the next person at
  the fork sees the rule before they write.
- A test that fails if the rule's sentence leaves the controlled document.

## What this does not do

It does not make the SAS compile through `war`. That the two are the same
class of artifact is a statement about their structure and governance, and
this Warrant records it as normative text; a SAS profile for the compiler is
a later Warrant.
