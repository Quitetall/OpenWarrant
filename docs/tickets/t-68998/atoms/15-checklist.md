# Checklist

- [x] Close the writer admission race with an atomic lock and regression evidence (i-a689) — done by agent://codex, 2026-10-11: Closed probe-to-spawn race with non-blocking checkout admission held through reap; contention only observes writer records. Clean-base reproduction started five overlapping writers; fixed standalone run starts one without overlap. All 19 selected cancellation, handoff, adapter and concurrency checks pass. Full gate and independent acceptance remain separate.
