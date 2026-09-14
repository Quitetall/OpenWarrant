---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e81-7722-b56c-53f89968bd3c
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the bounded result is observed
- **scope:** Candidate source-set review and the existing SAS acceptance boundary; Exact RC.2 adoption and compatibility basis.
- **evidence:** Recompute all candidate manifest entries; demonstrate that the proposed adoption record binds the SAS, format contract, schemas and fixtures. Independently review the legacy-to-RC.2 mapping. The authorized human accepts the exact candidate and adoption ADR through the approved procedure.

### OBL-002 — the controls reject their stated counterexamples
- **scope:** the refusal cases within Candidate source-set review and the existing SAS acceptance boundary.
- **evidence:** A changed companion contract or manifest must invalidate the reviewed basis. An old 1.0.0 signature must not count as RC.2 acceptance. A proposal over the currently configured legacy SAS must not be mislabeled RC.2.

### OBL-003 — independent evidence supports acceptance of the exact result
- **scope:** this Warrant's final deliverable revision and the two obligations above.
- **evidence:** a separate verifier reproduces required observations, reviews exact source/fixture/build digests and reports bounded findings; the authorized human accepts or resolves through the applicable process. Performer tests alone cannot establish this obligation.

### OBL-004 — the delivered Warrant records remain structurally valid
- **scope:** this Warrant's authored records and their current repository references; no claim of runtime feature correctness.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a recorded run of the qualified structural gate on the exact deliverable revision, with its existing planted controls retained. A named fixture/schema violation is refused in a disposable corpus; a structural pass cannot satisfy OBL-001 or OBL-002 by itself.

## Gate Adequacy

All observations above are required and currently NOT RUN for this new Warrant.
No disposition is declared. `war check` checks draft structure, not feature
correctness. `war check --generated` checks generated consistency, not runtime
conformance. Functional commands must capture command, exit, exact inputs,
observed output and revision; an expected refusal passes its test only when
the named refusal is observed. Missing cases and unknown checks cannot pass.

For library slices, use the documented future rc2_probe --feature command and
public-boundary integration tests. For phase exits, run the documented full
inventory and cargo xtask gate on the pinned toolchain. Register/qualify any
new executable gate before citing it as an authoritative gate result; this
draft invents no gate qualification. Evidence admissibility and independent
verification remain prerequisites for resolution in addition to the separately bounded structural gate above.

**Adversarial question:** can constant success, a fixture lookup table, copied
performer assertions, or an unobserved check satisfy these obligations? No:
the named mutations/refusals, exact input identities and independent reproduction
must establish the bounded behavior. Failures remain visible until repaired or
properly dispositioned by an authorized independent actor.
