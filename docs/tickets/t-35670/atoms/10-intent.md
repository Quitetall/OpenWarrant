# M13: optional parts and the tick ladder

## Notes

- **2026-10-07 12:49 UTC, claude:** Started M13 in /mnt/4tb/tmp/claude-m13 (branch claude/m13, base 6db6dbf5). Design: optional parts live in a fixed-path optional atom atoms/30-checks.md (Tests, KPIs, Milestones sections), absent from every ticket that has none; a tick's level rides in the done suffix as a marker after the date (' [observed]'), which older parsers read as part of the date, so claimed ticks keep today's bytes.
