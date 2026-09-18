# Question reader integrity

Original CLI fixture: replacing questions/ with a file changed one blocking
question into zero without an error for questions/answers. Inbox already refused.
Regression observed red before fix. Missing storage is now distinct from failed
enumeration. Complete reads no longer discard errors; partial listing/MCP retains
healthy records and failure diagnostics. No authority or execution behavior granted.

Targeted CLI, MCP and inbox tests pass. Clippy and generated checks pass. Full gate at 1f6ebd9d97aae17a3bbb9816308d23801a94d64a passed 14/14 steps
and 308 planted controls on Rust 1.97.1 in an isolated clone. Implementation
complete/unverified. Merge CI and independent assurance remain separate. This is not the runtime hotline implementation.
