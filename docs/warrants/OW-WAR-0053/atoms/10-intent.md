---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1fce-7952-8d99-048355e95d11
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Carry the six research ADRs that are in flight and do not block, and are not
blocked by, the topology or data tracks.

    0009  Latent dimension scaling — empirical architecture decision
    0116  LamQuant Optimum v2 as a frozen learned lossless peer codec
    0054  Warplan — beat ITU-T H.BWC on its own datasets and metrics
    0077  Lossless paper venue — TBioCAS -> IEEE TBME
    0015  OpenHuman Eagle — EEG validation platform spec
    0060  Research subtree — speculation/survey ledger

## Why these are one Warrant and not six

They share no dependency edges with each other, which is exactly why they belong
together: this Warrant exists so the other two tracks are not blocked waiting on
research, and so research is not blocked waiting on a repository collapse.

There is NO required internal order. Sequence by scheduling reality, not by
dependency — 0077 is calendar-driven (submission dates), 0009 and 0116 are
GPU-bound and contend for the same box, 0060 is housekeeping that can fill gaps.

## The one hard constraint

GPU and RAM are shared with a self-hosted CI runner and with each other. The
latent-64 teacher sits near 35 GB of 62, and the standing rule is that big jobs
run ALONE. Two of these ADRs cannot be in flight simultaneously if both train.

## What this Warrant must not become

A place to relitigate settled negative results. The don't-retry list is real and
was paid for: grow-decoder, +lambda-R, INR, Muon with ternary QAT, BitNet, a
generative head, V2 depthwise-separable (-0.026 R vs V1), and E2 absolute
high-frequency amplitude loss (killed — it fits noise on gamma). Retrying one of
these needs a stated reason the previous measurement was wrong, not a hunch that
it might work this time.
