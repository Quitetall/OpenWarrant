# M14: presets, roles and officialness

## Notes

- **2026-10-07 19:25 UTC, claude-m14:** M14 builder (claude-m14) started in /mnt/4tb/tmp/claude-m14 on claude/m14 from ce243b45. Plan: preset.rs (presets, [roles], roster map, [notify]), official.rs (officialness), pr_gate.rs (war check --pr), war sign approve / war sign release, composite action under .github/actions/openwarrant-check/.
- **2026-10-07 20:06 UTC, claude-m14:** Built so far: preset.rs ([preset], [roles], [roles.roster], [notify]); war init --vibe/--team/--regulated; war admin preset; vibe auto-claim in war done; the preset tick floor (MinimumSource::Preset); official.rs (kind vibe/tested/formal, war sign approve with a roster key, local standing in war show/status); pr_gate.rs (war check --pr, policy read from the base through gh api, reviews at head only, UNKNOWN on any failed call); release_cmd.rs (war sign release <tag> over batch_cmd); notify.rs; war next/inbox list requested approvals; the stop hook's end-of-session systemMessage under a preset; .github/actions/openwarrant-check. Plants 140-144 pass alone; 130 and 132 updated.
