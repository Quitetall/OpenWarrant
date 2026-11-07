Fixtures for `conformance/plants.d/220-session-usage.sh`: what a Claude Code session
appends to its transcript, never a real one. `before.jsonl` is already in the
transcript when `war start` marks it; `after.jsonl` and `subagent.jsonl` are appended
after. Every message carries `FIXTURE-SECRET` content, which no record may hold.
