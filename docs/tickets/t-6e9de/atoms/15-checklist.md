# Checklist

- [x] Add pure reservation and settlement replay with poisoned-state recovery and refusal controls (i-8fd1) — done by agent://codex, 2026-10-14: Implemented pure run history and deterministic restore: held and unknown reservations retained, malformed histories and changed policies refused, breach/overflow recovered poisoned and repeat overflow settlement refused. Eight spend controls and all 751 core tests pass; strict Clippy/fmt pass. Host history authentication/durability and real provider accounting remain open; no independent qualification or paid calls.
