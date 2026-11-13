# Over MCP

`war admin mcp` serves the CLI to any harness over stdio (`war admin mcp --describe`
prints the table). Ordinary work needs none of these tools.

By default it serves the daily set: `war_start`, `war_done`, `war_create`,
`war_status` (`next`, `ready`, `target` with `view` fold `war_next`,
`war_ready` and `war_show`), `war_check`, `war_context` and `war_note`.
`war_start` takes `claim` (the old `war_claim`); `war_done` takes `release`
and `reopen` with `reason`.

`war_start` automatically returns its complete required task packet in the
report envelope, in both structured content and a text fallback. This also
applies with `json: false`. A claim-only call reserves work without starting
it. Other replies use the CLI's text. A finding reads
`refused (rule): ...` and a nonzero exit ends `exit N, not_ready`. Pass
`json: true` for the `oh.war/report/v1` envelope (or set `[mcp] structured =
true`).

`war admin mcp --all` (or `[mcp] tools = "all"`) serves the whole table:

| kind | tools |
|---|---|
| work loop (human acts stay with humans) | `war_start`, `war_done`, `war_prime`, `war_ready`, `war_claim`, `war_create`, `war_add`, `war_note`, `war_show`, `war_tickets`, `war_heartbeat` |
| read-only | `war_context`, `war_check`, `war_status`, `war_journal`, `war_diff`, `war_gate_list`, `war_sas_status`, `war_resolve_dry_run`, `war_sign_list`, `war_sign_show`, `war_pins`, `war_next`, `war_questions`, `war_answers`, `war_standing_show` |
| request halves (write nothing) | `war_authorize_request`, `war_resolve_request`, `war_verify_request`, `war_sas_accept_request`, `war_plan_request`, `war_plan_validate` |
| writes an agent makes | `war_new`, `war_deliver`, `war_evidence_record`, `war_compile`, `war_gate_run`, `war_journal_backfill`, `war_dispatch` (journals the dispatch), `war_ask`, `war_standing_apply`, `war_plan_apply` (reviewed=true only) |

Not registered, so calling one is "tool not found": any signing act, any
`--response` ingest, `sas propose`, `kf`, `telemetry`, `migrate`, `export`,
`bonsai`, `init`, `gate --record`, `plan --draft`, and the separate `war_answer`
tool. Questions are answered through `war_note` when the work's requirements
permit it. Signing is a person's act at a terminal.

Resources: `warrant://<alias>`, `warrant://<alias>/status`,
`warrant://<alias>/journal`, `context://index`, `context://<target>`, `status://corpus`, `sas://current`, `pins://all`,
`next://`.

Client config (the plugin ships this in `.mcp.json`):

```json
{"mcpServers": {"openwarrant": {"command": "war", "args": ["admin", "mcp"]}}}
```
