---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f7b-7680-8fa7-f84f8dda0962
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

Measured in `/mnt/4tb/LamQuant` on 2026-08-26. Re-measure before acting.

## Status and gates

All four are `status: in-progress`. Three carry a `gate_cmd`; one does not:

    0159  in-progress  gate_cmd: yes
    0158  in-progress  gate_cmd: yes
    0074  in-progress  gate_cmd: yes
    0075  in-progress  gate_cmd: NO

0075 having no gate is a finding, not an oversight to fix by inventing one.
ADR 0179 forbids elaboration before execution, and ADR 0186 clause 4 is explicit
that writing tools to satisfy documents rather than needs is the wrong repair.
Decide whether 0075 needs a gate because the WORK needs one — not because the
other three have one.

## Corpus-wide gate health, for calibration

`tools/adr-gate-ceilings.json`, 94 classified:

    PASS 43 · FAIL 26 · MISSING-TOOL 12 · MISSING-SCRIPT 7 · MISSING-CRATE 4
    MUTATING 1 · TIMEOUT 1

The 24 MISSING-* are gates that cannot be ASKED, which is a different fact from
failing. Do not collapse "could not ask" into "failed" — §96.4 carries them as
distinct classes for exactly this reason.

`tools/adr_closure_debt.toml` currently has 0 entries.

## Storage constraints that bound this work

- Canonical data lives at `/mnt/4tb/data`, one LMA per corpus, hash-verified.
  Never `rm` gitignored `ai_models/` checkpoints.
- The box is 62 GB with a 24 GB GPU. The latent-64 teacher sits near 35 GB, and
  the standing rule is that big jobs run ALONE. Check `free -g` and sum
  footprints before starting anything concurrent.
- A self-hosted CI runner now shares this machine (see LamQuant #132). It is
  interlocked on free RAM, GPU residency and load, and capped at 12 CPUs / 24 GB,
  but the interlock only gates when a job STARTS — it cannot see a trainer that
  begins mid-job. Expect contention; do not assume the box is idle.

## Blocking dependency outside these four

Six of eight training recipes need a sealed BCS2 snapshot that nothing currently
produces (LamQuant #131 lineage). That blocks ADR 0103's witness and the ADR 0109
study. If a step here needs a sealed snapshot, that gap is upstream of it and
must be closed first rather than worked around with an unsealed stand-in.
