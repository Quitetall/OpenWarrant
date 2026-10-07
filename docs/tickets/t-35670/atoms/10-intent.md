# M13: optional parts and the tick ladder

## Notes

- **2026-10-07 12:49 UTC, claude:** Started M13 in /mnt/4tb/tmp/claude-m13 (branch claude/m13, base 6db6dbf5). Design: optional parts live in a fixed-path optional atom atoms/30-checks.md (Tests, KPIs, Milestones sections), absent from every ticket that has none; a tick's level rides in the done suffix as a marker after the date (' [observed]'), which older parsers read as part of the date, so claimed ticks keep today's bytes.
- **2026-10-07 14:26 UTC, claude:** Built on claude/m13 (merged 4ec1a661). Gates: cargo test --workspace 1322 passed (TMPDIR on tmpfs: a stray /tmp/.git, not ours, makes the verification_subject_cli fixtures read as inside a git repository); clippy clean with and without --features schema; war schemas --check well-formed; plant 110 alone 26/26 in a throwaway clone; the ticket plants 45/46/55/72/73/75/77/78/100/101 alone 139/139. The hook falls back to a claimed tick when the item has nothing to check (war done --check would refuse check-nothing); chosen so a completed Claude Code task always feeds the ladder.
