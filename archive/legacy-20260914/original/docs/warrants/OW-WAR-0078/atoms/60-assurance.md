---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e8f-76f2-937e-b35825ad9382
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the bounded result is observed
- **scope:** evaluate_condition over declared stage, subsystem and path inputs; F04, T16, T17, T18, T19, T20.
- **evidence:** T16-T20 prove truth tables, missing versus explicitly empty fields, anchored glob segments, Unicode/case behavior and zero-segment **.

### OBL-002 — the controls reject their stated counterexamples
- **scope:** the refusal cases within evaluate_condition over declared stage, subsystem and path inputs.
- **evidence:** Unsupported operators, empty conditions and malformed globs produce condition-invalid rather than being downgraded to UNKNOWN or silently guessed.

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

## Planned direct acceptance commands

These commands become runnable only when this slice is implemented.

```bash
cargo run -p openwarrant-compiler --example rc2_probe -- --feature F04 --fixtures conformance/rc2
```
