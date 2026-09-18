# OW89 candidate packaging rehearsal

Linux and native macOS installation rehearsals now pass at the same source
revision. Release qualification remains blocked. No release,
qualification, signature or human acceptance is claimed.

Candidate bundles preserve the complete committed source layout, CLI, SDK,
standard, fixtures, adapted skills and license notices. Canonical links remain
usable after extraction. The installed CLI executes SDK validation and interactive
authoring, and its temporary installation is removed. Exact source, executable
and payload digests are in [the Linux receipt](linux-install.json).

Six refusal tests cover checksum/inventory tampering, duplicate fields/members,
false qualification, escaping/link entries, existing destinations, hidden PAX
metadata expansion and nested sparse-file expansion. Confirmed review defects
were fixed and independently rechecked. Full gate: 14/14 steps, 308/308 controls.
See [tests](tests.log), [gate](gate.log), and [source inventory](observation.json).

The archive preserves Git source bytes; it does not authenticate the supplied
binary's source linkage. Native release build provenance is still needed.
Dependency notice text omitted from rmcp packages is preserved from their exact
recorded upstream commit, with an offline hash/VCS check. Notice inventory is not
legal certification. Earlier relocated-source rehearsal was superseded after
review found broken context links. The final committed-source install passed.

Independent spec/standards review: PASS for bounded packaging implementation.
LAMU local commit review: PASS WITH NITS; alleged path/hard-link defects were
explicitly self-retracted by reviewer and checked against source. No paid call.
All tag publication remains blocked until OW90 qualification and OW91 permission.

## Native host observations, 2026-09-18

[Comparison and archive identities](native-20260918/comparison.json) bind both
native installation receipts to source `43a3c2f2a3704e9d439b2e2a1f360c49b5191ac6`.
Full receipts are retained as gzip files with their original byte hashes. Both
hosts ran installed skill-link checks, CLI version, SDK example and interactive
authoring. IR inventories match byte for byte. The workflow verified archive
round trips. Release creation and crate publication were skipped.

These observations close the missing native macOS rehearsal, not Phase 3 or
release qualification. Hosted execution records supply build context; they do
not establish a cryptographically signed source-to-binary attestation.
