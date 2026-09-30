---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e96-7523-b7c6-4b3c2a0e913f
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the bounded result is observed
- **scope:** assemble_master, project_role and render_entry over one versioned IR; F06, T27, T28, T29, T30, T31.
- **evidence:** T27-T31 prove common membership/basis across machine and human views, exact rule slices, retained evidence behind excerpts and provenance/trust label inheritance.

### OBL-002 — the controls reject their stated counterexamples
- **scope:** the refusal cases within assemble_master, project_role and render_entry over one versioned IR.
- **evidence:** A background summary cannot replace a binding rule; a performer claim cannot turn into a verification disposition; an excerpt cannot masquerade as complete evidence.

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
cargo run -p openwarrant-compiler --example rc2_probe -- --feature F06 --fixtures conformance/rc2
```
