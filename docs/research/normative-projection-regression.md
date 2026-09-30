# Normative projection regression

Date: 2026-09-15. Performer test report; not independent acceptance.

LAMU/KF integration review found valid lettered sections absent from both generated
normative views. The views agreed with each other but omitted source requirements.
The parser admitted only numeric level-two headings and unconditionally skipped
section 3. KF uses sections 8A and 104A; LAMU uses section 3 for architecture.

Two added tests failed before the fix: lettered sections produced zero sentences,
and a section-3 architecture requirement also produced zero. The fix admits
decimal components with one uppercase suffix, including nested 8A.1, and limits
the legacy section-3 exclusion to the heading "Normative language".

`cargo test -p openwarrant-core normative_tests` passes all six tests after the
fix. Existing language-definition, code-fence, table and lead-in behavior remains
covered. Negative identifier cases reject empty components, bare letters and
multiple suffix letters.

The built CLI regenerated downstream documents without changing their source
bytes. KF gained seven section-8A and thirteen section-104A sentences. LAMU gained
nine section-3 sentences. Both generated JSON and Markdown now include those
clauses. A projection is still a derived aid, not a substitute for the source SAS
or evidence of implementation conformance.

`cargo xtask gate` passed all 14 steps, including 308 planted-violation checks
with zero failures. This is local execution evidence on the pinned toolchain;
hosted CI and independent acceptance are separate observations.
