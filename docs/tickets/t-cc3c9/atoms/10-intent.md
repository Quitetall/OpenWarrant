# M16: CLAUDE.md and AGENTS.md

## Notes

- **2026-10-07 13:33 UTC, claude-m16:** M16 on claude/m16 (/mnt/4tb/tmp/claude-m16). Block: `war agents-md --block` (name kept), markers <!-- openwarrant:begin/end -->, stamp line inside the block so skew reads it unchanged; template ends with the same block so --block on a fresh init is a no-op. Writer is strict (two blocks, unterminated, stray end, unclosed trailing fence refused by name; nothing written to any file), reader lenient. Sections: core/instruction.rs reader (not the atom grammar); ids md:<file>#<slug>, GitHub slugs; block is a section boundary so restamping moves no revision.
