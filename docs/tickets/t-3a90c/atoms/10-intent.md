# Run a shared preparation gate once with all selected contracts bound before execution

## Notes

- **2026-10-14 23:56 UTC, agent://codex:** Additional public control changes the gate argv from success to false during execution. First attempt was UNKNOWN before gate setup completed: admin compile timed out at 90s under observed high I/O pressure; no deadline was widened. After pressure fell, fixture syntax was corrected for restricted YAML and the control reproduced a done outcome under a changed definition. Typed GateDefinition equality before/after execution now returns UNKNOWN with no current attachments; passing control retained. No receipt hashing recipe or wire fields changed. Full CI remains pending.
