# Reviewed subject binding: saved implementation slices

This is unfinished work for ticket t-b9610, saved in draft PR #138. It is not ready to merge or release. The representation decision is proposed in OW-ADR-0030; it is not adopted authority.

Historical verdicts remain on disk. Current preparation, resolution, progress and assurance-mark evaluation require the contract/artifact/gate/fixture snapshot bound by the verdict's journal event. Unbound or stale history is UNKNOWN. Known independence failures retain their specific inadmissibility reason.

The configured verifier wrapper echoes the subject from the bundle it actually read, using a private copy. Ingestion rejects stale subjects and undeclared obligation identifiers before writing verdicts. Malformed deliverable declarations cannot become an empty reviewed artifact set.

## Local evidence

Twelve public CLI regressions cover historical retention, fresh qualification, exact replay, changed artifact refusal without writes, progress/mark consistency, path escape through undeclared obligation identifiers, malformed declarations, the actual configured wrapper using a synthetic executable, known independence failures, direct stale-contract refusal, changed required fixtures, changed gate definitions and exact portable gate/fixture contents under forced per-obligation splitting. These are fixture observations, not live independent verification or model qualification. Five existing verification unit tests, seven mark unit tests and three CLI binary tests passed. Clippy over all CLI targets, formatting, shell syntax and the regenerated corpus check passed. No paid model ran.

The first independence fixture changed transcript blindness at basic assurance, which does not require that dimension. Source inspection corrected the fixture to change protected-fixture write isolation, which every assurance level requires. The tool was not weakened to satisfy the test.

## Remaining work

- Candidate-specific acceptance checks using exact Git source facts.
- Remaining context/evidence and declared gate-input coverage, the existing canonical bundle identity, and complete portable context closure.
- Unsafe-file and race controls, including multi-obligation processing.
- Conformance compatibility and complete gate, including regenerated projections.

No human approval, independent disposition, resolution or release is recorded by these implementation slices.

Workspace tests at commit 2d3ca6db passed: 1,200 tests across 44 completed suites. This predates the gate/fixture extension and does not establish the final branch gate. The newer twelve CLI checks cover the gate/fixture and portable-source extensions.

The portable bundle now carries `required_sources`: exact UTF-8 text for readable gate definitions and fixtures, or exact byte arrays for non-UTF-8 fixtures. Each includes its path, kind and ordinary SHA-256; missing fixture bytes remain explicit. These sources are fixed parts of every obligation packet and are not excerpted away to fit a budget. A disappeared, duplicated or changed required gate during capture refuses assembly. This is fixture/gate delivery, not complete context or execution sandboxing.

Existing verifier conformance observed one real adapter incompatibility: the fixture verifier omitted the new subject. Both synthetic adapters now echo the received subject, without reading newer repository state. The 62-verifier controls passed on the earlier production slice, and the regenerated 84-verify-bundle group passed all five checks. The first adapter rerun had one corpus projection-drift failure; source recompilation fixed it. These counts are separate observations and are not a final full-branch gate.
