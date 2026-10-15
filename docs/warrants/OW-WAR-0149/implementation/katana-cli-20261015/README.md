# Real CLI capture with patched TLS

[Katana PR #14](https://github.com/Quitetall/katana/pull/14) is merged. Its one-shot CLI can produce the existing native receipt
for OpenWarrant. [PR #15](https://github.com/Quitetall/katana/pull/15) updates rustls to 0.23.46 after four newly published
advisories broke the unchanged dependency-policy gate. Both the PR and patched
main CI pass. OpenWarrant's exact rustls pin and lockfile are updated to the same
patch without changing feature or license policy.

The repository now includes a [portable local producer](../../../../../conformance/fixtures/runtime-capture/katana_cli_fixture.py)
and [repeat instructions](../../../../../conformance/fixtures/runtime-capture/README.md).
It executes the actual Katana CLI and a real bounded file read. Its two model
responses are scripted SSE served on localhost; it calls no AI model.

Local observations on Rust 1.97.1: four OpenWarrant TLS controls pass, including
certificate hostname refusal. The existing native capture/current-resolution
control passes using this producer; excess capabilities, changed logs, a wrong
head and false assurance refuse. Required confinement, mandatory cost accounting
and absent native logs remain UNKNOWN. Advisory and all-feature license checks
pass. Katana's Rust 1.99.0 observations retain six public CLI and eight native
receipt passes; one instrumentation child is invoked by its parent control.

`observation.json` names exact provider sources and binaries. The provider build
uses da6553d, merged with the same tree as 7b8de536. The OpenWarrant test workspace
starts at 8de5f0c7 plus the manifest/lockfile and producer changes in this change.
The source patch and fixture hash are retained; the observations are not a claim
about an unmodified 8de5f0c7 or any later version. Raw synthetic binding, receipt
and session bytes before negative mutations are in `positive-cli-captures.tar.gz`.
All original logs remain compressed without changing their decompressed bytes.

The fixture's log head is test instrumentation, not protected production custody.
These are performer observations, not real coding-agent qualification, independent
verification, participant acceptance, Warrant resolution, phase exit or human
assurance. BLUT's contributor-agreement gate and remaining operator, fencing,
confinement and spend qualification remain open. Earlier observations and the
failed pre-patch provider CI are preserved.

Primary advisories: [0346](https://rustsec.org/advisories/RUSTSEC-2026-0346.html),
[0347](https://rustsec.org/advisories/RUSTSEC-2026-0347.html),
[0348](https://rustsec.org/advisories/RUSTSEC-2026-0348.html),
[0349](https://rustsec.org/advisories/RUSTSEC-2026-0349.html).
