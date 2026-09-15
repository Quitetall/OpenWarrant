# Over MCP

`war mcp` serves the same surface to any harness over stdio (`war mcp
--describe` prints it). Every tool answers with the `oh.war/report/v1`
envelope as structured content.

| kind | tools |
|---|---|
| read-only | `war_check`, `war_status`, `war_show`, `war_journal`, `war_diff`, `war_gate_list`, `war_sas_status`, `war_resolve_dry_run`, `war_sign_list`, `war_sign_show`, `war_pins`, `war_next`, `war_dispatch` |
| request halves (write nothing) | `war_authorize_request`, `war_resolve_request`, `war_verify_request`, `war_sas_accept_request`, `war_plan_request`, `war_plan_validate` |
| writes you may make | `war_new`, `war_evidence_record`, `war_compile`, `war_gate_run`, `war_journal_backfill`, `war_plan_apply` (reviewed=true only) |

Never registered, and calling one is "tool not found": any signing act, any
`--response` ingest, `sas propose`, `kf`, `telemetry`, `migrate`, `export`,
`bonsai`, `init`, `gate --record`, `plan --draft`.

Resources: `warrant://<alias>`, `warrant://<alias>/status`,
`warrant://<alias>/journal`, `status://corpus`, `sas://current`, `pins://all`,
`next://`.

Client config (the plugin ships this in `.mcp.json`):

```json
{"mcpServers": {"openwarrant": {"command": "war", "args": ["mcp"]}}}
```
