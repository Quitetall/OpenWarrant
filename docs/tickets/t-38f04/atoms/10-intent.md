# M18: every development document is a type

## Notes

- **2026-10-07 19:47 UTC, claude-m18:** M18 build started in /mnt/4tb/tmp/claude-m18 (claude/m18 from ce243b45). Design: roadmap, spec and adr are document types with an `encoding` (roadmap|sas|adr), a closed set of kernel store readers; a store type may select acceptance where its store records a human act (roadmap, sas), never authorization/verification. The three are built in (include_str of profiles/*.toml) and a program's own file replaces one; their check rules are gated on the type's capabilities. Differential on one tree, base vs new binary: roadmap, sas status, check, check --json, check --generated, status, status --json byte-identical; only war model gains records (524: 491 SAS sections, 31 ADRs, OW-ROADMAP, WAR-SAS) and relations.
