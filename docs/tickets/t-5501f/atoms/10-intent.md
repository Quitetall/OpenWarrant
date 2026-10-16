# Prepare alpha.4 preview with current integration fixes

## Notes

- **2026-10-15 22:05 UTC, agent://codex:** Fresh CI38800417327 exposed stale plugin version and the template-generated legacy guidance stamp after the CLI version bump. Plugin metadata now matches alpha.4; guidance regenerated with the actual alpha.4 CLI. Both exact regression tests pass. Original failed run retained; source projections and fresh CI still required.
- **2026-10-16 00:26 UTC, agent://codex:** Release 38819931497 passed the full gate and both native builds, then both package steps refused Bundle payload limit exceeded. Frozen source 7ab1e6a has a 264755200-byte full-source tar against 268435456-byte payload cap. A proposed v2 capsule keeps all source as a bounded gzip member; the 0.66-second real-Git fixture reproduced the same refusal before the patch. Twenty-five release tests now pass, including exact Git tar bytes, source paths, executable mode, an internal symlink, old-v1 support, nested expansion refusal, corruption, ambiguity and unsupported-version controls. This is a prototype fix, not native qualification or publication; no source or history was removed.
