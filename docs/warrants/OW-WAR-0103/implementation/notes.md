# Question reader integrity

Original CLI fixture: replacing questions/ with a file changed one blocking
question into zero without an error for questions/answers. Inbox already refused.
Regression observed red before fix. Missing storage is now distinct from failed
enumeration. Complete reads no longer discard errors; partial listing/MCP retains
healthy records and failure diagnostics. No authority or execution behavior granted.

Targeted CLI, MCP and inbox tests pass. Clippy and generated checks pass. Full gate remains pending. This is not the runtime hotline implementation.
