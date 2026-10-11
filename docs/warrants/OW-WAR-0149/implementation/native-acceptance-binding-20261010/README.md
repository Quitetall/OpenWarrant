# Native acceptance binding — draft implementation checkpoint

Unverified work under OW149. No human acceptance or independent verdict.

The draft native API binds exact selected captures, protected policy, contract,
assurance and artifact snapshots, decision fields and a clean Git locator. It
reassesses current inputs before ingestion, refuses replacement or signature
supplementation, and stores the original human-signed response bytes. This API
is not yet exposed by command-line signing or response ingestion.

Record readers reject legacy/native hybrids and mismatched signed decision
fields. The signature reader uses the actual protected principal identity,
not the actor's display name. Unsupported versions remain UNKNOWN. An ingestion
timestamp is service metadata; it cannot replace the signed effective time.
Historical signature validation is not a fresh execution verdict or assurance.

Three parser/record tests, two actual OpenSSH software-fixture transport tests,
and three legacy authority regressions passed. The software signatures establish
neither human presence nor this project's acceptance. Final formatting and CLI
all-target/all-feature Clippy passed. The first formatting check failed on one
layout and was corrected; its log remains. The clean-source schema-enabled build
and v1 schema/TypeScript pack check passed. Native draft formats stay outside the
published v1 pack.

The full positive native preparation/ingestion round trip, stale-input controls,
human signing flow and command-line integration still need implementation tests.
No existing resolution was rewritten. Checks around input reads and target CAS
are not an atomic fence over all source files. Production operator, hardware,
spend, participant and independent qualification remain separate work.

Policy PR211 passed its full gate and merged normally into main at
`6a9a159053cebc60facb077825b0cacacb035de2`. Its merge into this branch preserved
the tested source tree. `observations.json` records revisions, limits and log
hashes. OW95 has no participants; no session or result was invented.
