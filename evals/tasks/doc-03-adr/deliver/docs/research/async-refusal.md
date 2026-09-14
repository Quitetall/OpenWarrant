# Async outside MCP

## Decision

Refused. The MCP transport is the one consumer; a runtime elsewhere would add 22 crates for no caller.

## Alternatives

- Sync stdio with a thread per stream: rejected by the rmcp API.
