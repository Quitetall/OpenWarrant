# Provider runtime reader and exact stage bindings

Knowledge Fabric draft PR #5, head `888dbb69`, adds an authenticated offline
runtime evidence reader. It checks the signed provider package with configured
historical public keys and preserves selected contract, dispatch and receipt rows,
including exact PostgreSQL JSONB wrappers. Optional OpenWarrant dispatch packets
are checked using the existing `oh.war/dispatch/v1` digest domain, Warrant identity,
contract revision and contract digest. Missing mappings remain explicit.

Four local tests passed, including real PostgreSQL/MinIO source shutdown and the
Rust-produced OW75 packet retained here. Package build, ESLint and test TypeScript
compilation passed. Hosted qualification is pending. This reader does not establish
native runtime semantics or complete project-stage coverage. Archive integration
and real runtime proof remain open.

The fixture command was `war dispatch OW-WAR-0075 STAGE-001 --emit <packet> --json`.
It compiled a packet; it did not launch work. The CLI also appended draft-history
event `01a0b552-8e78-72f2-8209-cca5556b4239` to OW75's journal. That actual event is
preserved, and OW111's unsigned scope now includes this exact journal path.
The CLI recorded configured actor `agent://claude`; the invoking assistant was
Codex. This is a tool-attribution mismatch, not evidence of a Claude execution or
human act. The raw record is not rewritten to conceal the mismatch.

A prior OW30 compilation request refused because its stage has no executor_ref.
Its source records were not changed to force compilation. Producer and packet
hashes are retained in `ow75-dispatch-identity.json`. No assurance disposition,
contract authorization or resolution is created by these observations.
