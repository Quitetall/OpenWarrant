---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1fce-7952-8d99-048355e95d11
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

Measured in `/mnt/4tb/LamQuant` on 2026-08-26. Re-measure before acting.

## Status and gates

    0009  in-progress  gate_cmd: NO
    0015  in-progress  gate_cmd: NO
    0054  in-progress  gate_cmd: yes
    0060  in-progress  gate_cmd: yes
    0077  in-progress  gate_cmd: yes
    0116  in-progress  gate_cmd: yes

Two carry no gate. As with OW-WAR-0052, treat that as a fact to decide about
rather than a hole to plug reflexively — ADR 0186 clause 4 and ADR 0179 both
reject writing tools to satisfy documents.

## Numbers come from the ledger, not from memory

`docs/TRUTH_LEDGER.md` §2 is authoritative. Never retype a ledger number into
prose — write `{{ledger:ID}}` and let the composer transclude it; it fails closed
on an unknown id.

Standing corrections that keep being re-asserted and are FALSE: firmware boots
(#170), LMQ ships, R = 0.81 / 0.85 / 0.93, FDA clearance, 64 KB SRAM (it is
520 KB). Honest R is roughly 0.42-0.52 fullband; LQS-M is R >= 0.85 and
PRD <= 20 per ADR 0043, NOT R 0.98.

All R figures to date are measured FSQ-OFF, i.e. continuous-latent. The
finite-rate gap is small (real is about 0.69 at 83x); the FP32 ceiling is 0.72
and ternary costs about 0.04.

The 0.77 -> 0.47 drop that recurs in old notes is a VALIDATION SET change, not a
model regression. Do not chase it.

## Prior art that bounds 0116 and 0054

- BLX1 achieves full peer parity against H.BWC by splitting the LSB plane before
  prediction: 7/7 corpora, 2.457% smaller. It SUPERSEDES the earlier H.BWC
  campaign's niche 5/8 verdict. File an IP disclosure before publishing.
- The H.BWC BSD-3 Rust clone passes conformance, but its patent non-grant means
  shipping is deferred. Conformance is not permission.
- 0077's pivot is settled: TBioCAS declined it as algorithmic rather than
  circuits; the target is TBME, hardware-first. Backup tag
  `paper-tbiocas-backup-2026-06-01`.

## Compute reality

62 GB box, 24 GB GPU (4090), 20 cores. Idle desktop already holds ~2.4 GB of
VRAM. A self-hosted CI runner shares the machine under an interlock that gates
job START only — it cannot see a trainer that begins mid-job.

Every training run goes through the BLUT cookbook recipe, never raw
`tools/*.sh`. Use `read_metric` for claims. A poller has fabricated a val-R
trajectory before; verify against raw journald or CSV, never a prose digest.
