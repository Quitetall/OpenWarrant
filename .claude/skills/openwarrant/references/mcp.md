# Over MCP

`war mcp` serves the CLI to any harness over stdio (`war mcp --describe`
prints the table). Every tool answers with the `oh.war/report/v1` envelope as
structured content. Ordinary work needs none of these tools.

| kind | tools |
|---|---|
| tickets (no signature anywhere) | `war_prime`, `war_ready`, `war_claim`, `war_done`, `war_create`, `war_add`, `war_note`, `war_show`, `war_tickets` |
| read-only | `war_check`, `war_status`, `war_show`, `war_journal`, `war_diff`, `war_gate_list`, `war_sas_status`, `war_resolve_dry_run`, `war_sign_list`, `war_sign_show`, `war_pins`, `war_next`, `war_questions`, `war_answers`, `war_standing_show` |
| request halves (write nothing) | `war_authorize_request`, `war_resolve_request`, `war_verify_request`, `war_sas_accept_request`, `war_plan_request`, `war_plan_validate` |
| writes an agent makes | `war_new`, `war_deliver`, `war_evidence_record`, `war_compile`, `war_gate_run`, `war_journal_backfill`, `war_dispatch` (journals the dispatch), `war_ask`, `war_standing_apply`, `war_plan_apply` (reviewed=true only) |

Not registered, so calling one is "tool not found": any signing act, any
`--response` ingest, `sas propose`, `kf`, `telemetry`, `migrate`, `export`,
`bonsai`, `init`, `gate --record`, `plan --draft`, `answer`. Signing is a
person's act at a terminal.

Resources: `warrant://<alias>`, `warrant://<alias>/status`,
`warrant://<alias>/journal`, `status://corpus`, `sas://current`, `pins://all`,
`next://`.

Client config (the plugin ships this in `.mcp.json`):

```json
{"mcpServers": {"openwarrant": {"command": "war", "args": ["mcp"]}}}
```
