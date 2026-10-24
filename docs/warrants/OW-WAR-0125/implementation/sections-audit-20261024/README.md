# W125 section-identity implementation audit

Step one derives section identities from the pinned SAS. Step two, authored SAS
sections, belongs to later work. This audit implements no new source format and
accepts no ADR or SAS revision. OW-ADR-0028 remains proposed.

Ten relevant source, ADR and CI files are byte-identical to protected CI head
`1f0fa2e34ef608073ac439653dd723ac9609c7b1`, run 40033704288, and to the sealed
consumer-tool source. The retained controls cover all five originally named
revisions plus three later ones; lossless joins and a changed-byte refusal;
fenced/unfenced headings; section-index digests and hand-edit refusals for both
views; nonexistent sections/subsections; changed-section warnings and shallow-
history UNKNOWN; self-diff and one-word changes; requirement-row removal;
proposed ADR status and its negative control; and no SAS delivery, with a
planted-delivery refusal. Full-history CI checkout remains configured.

## Exact historical scope

The first delivery commit `9c3c9150264f8b023e67e1035760521aeb518e58` retains SAS
1.1.0 at its recorded SHA-256. A read-only historical checkout, checked by the
sealed tool, returned 1,623 pass, 179 warn, zero unknown/error. All 43 citations
then present pass both reference and currency checks: the original sixteen and
27 later section 44.8 references identified in the ADR's own context.

Fifteen original citations compare 0.1.0-draft.1 with 1.1.0. OW-WAR-0071's
section 101 citation actually compares 1.0.0 with 1.1.0. That real pin is preserved.
A separate public SAS diff of the retained 0.1.0-draft.1 bytes against 1.1.0 names
ten changed parent sections and no added/removed units; none of the five parent
sections cited by the original sixteen changed. This also covers every cited
subsection without pretending OW-WAR-0071 had another pin.

That diff returns exit2 because the older candidate removes newer requirement
rows. Its requirement refusal is retained; only its separate section comparison
is used here. It is not described as an all-pass candidate. Full historical raw
reports remain at the durable paths named and hashed by the observation.
The historical source bytes and tracked tree stayed unchanged.

Implementation is reported complete, unverified. No signed atom, revision,
authority, deliverable pin, independent verdict or resolution was changed.
The current 1.3.0 test's 100 current/two stale citation result is not substituted
for the exact 1.1.0 historical check. Independent assessment and human acceptance
remain separate.
