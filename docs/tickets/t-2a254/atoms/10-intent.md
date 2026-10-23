# Preserve prior gate streams while a new gate is running

Isolated public CLI reproduction: direct hash-read control passes; the same argv under gate --run sees prior stdout already truncated and fails. Exact proof: /mnt/4tb/artifacts/OpenWarrant/keep/ow75-fresh-review-20261020/gate-stream-repro-v2/. CLI gate_cmd.rs has historical OW20/D002 pin governed by current OW133/D001. Preserve askability, deadline, stream refs, failed-run truth, and I/O refusals. Do not change core gate.rs or signatures.
