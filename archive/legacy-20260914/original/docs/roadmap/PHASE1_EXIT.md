# Phase 1 exit — measured

SAS §98, Phase 1 (file-native WAR compiler). Exit: **"OpenWarrant
development uses WARs."** This file is OW-WAR-0061's deliverable: each line
carries the command that produced it. Measured 2026-09-02 on `main`; re-measured 2026-09-03 after OW-WAR-0059–0061 were authorized and eight Warrants resolved (the verifier refuted the first count, correctly: it was stale).

## The ten deliverables

| §98 deliverable | Where it lives | Exercised by |
|---|---|---|
| `war init` | `crates/openwarrant-cli/src/init.rs` | `war --help`; conformance plants |
| `war new` | `crates/openwarrant-cli/src/new.rs` | every Warrant in the corpus was allocated by it |
| manifest | `crates/openwarrant-core/src/manifest.rs` | `war check` on 56 manifests |
| authored atom profile | `crates/openwarrant-core/src/atom.rs`, restricted frontmatter reader (OW-ADR-0002) | `war check` |
| canonical IR | `crates/openwarrant-compiler/src/{ir,lower}.rs` | every committed `generated/WAR.json` |
| `war check` | `crates/openwarrant-cli/src/check.rs` | the `corpus` gate step, every push |
| `war compile` | `crates/openwarrant-cli/src/compile.rs` | drift check of every committed view |
| full Markdown parent | `crates/openwarrant-compiler/src/render.rs` | `generated/WAR.md` ×56, drift-checked |
| canonical JSON | `crates/openwarrant-compiler/src/canonical.rs` (RFC 8785, OW-ADR-0001) | `generated/WAR.json` ×56, drift-checked |
| generated drift gate | `war check --generated` as the `corpus` step of `cargo xtask gate` | CI on every push since #41 |

```bash
./target/debug/war --help | grep -cE '^  (init|new|check|compile) '   # 4
ls docs/warrants/OW-WAR-00*/generated/WAR.json | wc -l                 # 57
./target/debug/war check --generated >/dev/null; echo $?               # 0
```

## The corpus

```bash
ls -d docs/warrants/OW-WAR-00*/ | wc -l                                # 57 Warrants
ls docs/warrants/OW-WAR-00*/authorization.toml | wc -l                 # 57 authorized — every Warrant in the corpus
ls docs/warrants/OW-WAR-00*/resolution.toml | wc -l                    # 8 resolved, each citing a bound §44.6 receipt
```

## Commit traceability (recorded, not passed)

```bash
git log --format=%s main | wc -l                                       # 91
git log --format=%s main | grep -c 'OW-WAR-'                           # 24
```

24 of 91 commit subjects name a Warrant. The others name a
SAS section, a fix, a records batch or a dependency bump. This is the
starting number for a stricter rule this repository has not adopted; a gate
that enforces it needs commit history CI's shallow checkout does not fetch.
