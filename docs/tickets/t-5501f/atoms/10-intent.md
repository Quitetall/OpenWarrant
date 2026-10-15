# Prepare alpha.4 preview with current integration fixes

## Notes

- **2026-10-15 22:05 UTC, agent://codex:** Fresh CI38800417327 exposed stale plugin version and the template-generated legacy guidance stamp after the CLI version bump. Plugin metadata now matches alpha.4; guidance regenerated with the actual alpha.4 CLI. Both exact regression tests pass. Original failed run retained; source projections and fresh CI still required.
