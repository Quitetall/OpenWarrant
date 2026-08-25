## What this changes

<!-- Describe the DIFF, not the state of the tree. -->

## Warrant

<!-- Report-phase evidence binds this change to a Warrant. Authorization remains
     a separate recorded human act; `war new` creates a draft only. -->

Warrant: OW-WAR-____

## Checklist

- [ ] `cargo xtask gate` passes (fmt, clippy, tests, licenses, planted violations)
- [ ] Any new control has a planted violation proving it rejects
- [ ] `war compile && war check --generated` — projections recompiled, no drift
- [ ] Any normative decision is recorded as an ADR under `docs/adr/atoms/`
- [ ] No new dependency, or the new dependency is MIT and/or Apache-2.0

## Toolchain

<!-- Output of `rustc --version`. A gate claim without a toolchain is not a claim. -->
