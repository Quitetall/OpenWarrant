---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f34-92db-54b2dca5446d
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a valid manifest parses into typed values
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** the §61 manifest shape as exercised by the five Warrants in
  `docs/warrants/`. No claim about manifests using fields none of them use.
- **evidence:** parse results asserted field by field.

### OBL-002 — each in-scope malformed composition is REFUSED
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** §91.2 tests 7 (missing required atom), 8 (duplicate ordinal),
  9 (unknown required role), and 12 (composition cycle).
- **evidence:** four planted violations, four observed refusals, each naming the
  offending file and rule.
- **note:** this obligation is about the validator's ability to say no. A
  validator returning `Ok(())` unconditionally satisfies OBL-001 completely.

### OBL-003 — the acceptance corpus parses
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** all five Warrants under `docs/warrants/`, universally — every one,
  not a sample (§38.4).
- **evidence:** a test that enumerates the directory rather than listing paths,
  so a sixth Warrant is covered the moment it is added and a deleted one causes
  a visible count change rather than silent under-coverage.

### OBL-004 — alias allocation is atomic
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** concurrent `war new` on one repository, same filesystem.
- **evidence:** N concurrent invocations produce N distinct aliases, or fewer
  aliases and a corresponding number of explicit failures. Never two records at
  one alias.

## Gate Adequacy

The adversarial question: **could OBL-001 through OBL-004 pass while a Warrant
that should be rejected is accepted?**

Yes, in one way that is being accepted knowingly. OBL-002 covers four specific
malformed compositions. §91.2 lists sixteen, and tests 10, 11, 13, 14, 15, and 16
are out of this Warrant's scope because they concern generated atoms, bound-atom
resolution, Source Holder ambiguity, and classification — none of which exist
yet. A manifest that is malformed in one of those six ways will parse here
without complaint. That is a real coverage gap, it is named rather than implied,
and it closes in OW-WAR-0003 and OW-WAR-0004.

## Residual Risk

- The frontmatter parser dependency is unchosen. If the resolution is to
  hand-roll a restricted reader, its input-handling becomes security-relevant
  under §87.2 and this Warrant's assurance level should be revisited before it
  resolves, not after.
