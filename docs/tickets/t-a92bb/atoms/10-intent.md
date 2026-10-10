# M12: a small surface

## Notes

- **2026-10-07 16:30 UTC, claude:** Started M12 in /mnt/4tb/tmp/claude-m12 (branch claude/m12, base 9dc413e2). Plan: group enums flattened a second time at the top level and hidden at run time (clap's own hidden subcommands), so every old spelling parses to the same variant and handler; one custom help template lists the daily verbs and a More: block.
- **2026-10-07 17:09 UTC, claude:** Messages now name canonical spellings: 688 suggestions rewritten in cli/core/compiler, plus remedy argv; remedy::classify reads the leaf through a group word; tests/suggested_commands.rs refuses a hidden spelling (files pinned by a resolved Warrant are exempt, read from war admin pins). Log prefixes (war ui: act ...), the TLS CN, git's merge driver line, schema TS header and adopt/SAS templates keep their bytes.
- **2026-10-07 17:34 UTC, claude:** First done is four daily verbs (init, create, claim, done), not three: war done needs your own claim (OW-WAR-0147 OBL-003), and M12 changes placement and help only. Plant 132 records the four-command path and both refusals; the three-command target needs an owner decision on OBL-003.
