# Reviewed subject binding: saved implementation slices

This is unfinished work for ticket t-b9610, saved in draft PR #138. It is not ready to merge or release. The representation decision is proposed in OW-ADR-0030; it is not adopted authority.

Historical verdicts remain on disk. Current preparation, resolution, progress and assurance-mark evaluation require the contract/artifact snapshot bound by the verdict's journal event. Unbound or stale history is UNKNOWN. Known independence failures retain their specific inadmissibility reason.

The configured verifier wrapper echoes the subject from the bundle it actually read, using a private copy. Ingestion rejects stale subjects and undeclared obligation identifiers before writing verdicts. Malformed deliverable declarations cannot become an empty reviewed artifact set.

## Local evidence

Nine public CLI regressions cover historical retention, fresh qualification, exact replay, changed artifact refusal without writes, progress/mark consistency, path escape through undeclared obligation identifiers, malformed declarations, the actual configured wrapper using a synthetic executable, and known independence failures. These are fixture observations, not live independent verification or model qualification. Five existing verification unit tests, seven mark unit tests and three CLI binary tests passed. Clippy over all CLI targets, formatting, shell syntax and the regenerated corpus check passed. No paid model ran.

The first independence fixture changed transcript blindness at basic assurance, which does not require that dimension. Source inspection corrected the fixture to change protected-fixture write isolation, which every assurance level requires. The tool was not weakened to satisfy the test.

## Remaining work

- Candidate-specific acceptance checks using exact Git source facts.
- Required fixture/context/evidence coverage and the existing canonical bundle identity.
- Direct stale-contract, unsafe-file and race controls, including multi-obligation processing.
- Conformance compatibility and complete gate, including regenerated projections.

No human approval, independent disposition, resolution or release is recorded by these implementation slices.
