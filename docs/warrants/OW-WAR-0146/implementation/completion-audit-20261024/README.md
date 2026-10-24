# W146 implementation completion

**Complete, unverified.** The exact UTF-8 fix candidate
`8606def37781037aa3741e8108b1d06bfb708445` passed protected CI
[40094082209](https://github.com/Quitetall/OpenWarrant/actions/runs/40094082209):
14 gate steps and 1,711 planted controls passed, zero failed. PR351 merged it
into main as `e2d1d76fb14aff0deb8a1351188c292a37f12a15`.

The [observation](observation.json) verifies that bundle.rs, the registered
47-bundle-output plant and verification_subject_cli.rs still have exactly the
tested bytes. The [retained plant excerpt](bundle-control-excerpt.log) covers
raw capture/tail values and whole-stream hash, truncation, missing capture,
legacy receipt digest absence, repeatable bundle identity and fake-verifier
response ingestion. The earlier UTF-8 red/green and stdout/stderr matrix retain
the additional rendered-byte boundary proof. Fake-verifier ingestion is a
fixture control, not independent verification of this Warrant.

The three original obligations and both implementation deliverables are covered
by those bounded source and runtime observations. The UTF-8 correction preserves
raw hash/byte accounting, receipt semantics and deterministic output while
limiting rendered text to the configured cap. No frozen record schema or gate
runner changed.

This completes the implementation report only. Historical delivery reconciliation,
admissible formal evidence, independent dispositions, risk authority and secure
human resolution remain open. The separate Bonsai report did not pass because
the immutable legacy contract has no bound scope; the protected required check
was `gate`. No contract sidecar, pin, signature or disposition was fabricated.
