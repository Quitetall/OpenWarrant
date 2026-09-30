# Checklist

- [x] Reuse tree observations within the one-shot board command without changing output or index bytes (i-f53b) — done by codex, 2026-09-30: Real CLI trace control observed 6 distinct Git scans once each, down from 1216; index bytes unchanged. Rust board parity and refusal test passed.
- [x] Observe authenticated HTTP board parity and retain the CI refusal reproduction (i-4bbc) — done by codex, 2026-09-30: Both targeted authenticated HTTP board tests pass with the original 10-second subprocess deadline. Full CI remains required after integration.
