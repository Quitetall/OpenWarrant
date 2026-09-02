---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f7b-7680-8fa7-f84f8dda0962
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Land the four in-progress ABIR/data ADRs in dependency order, so each is built on
a boundary that has stopped moving.

    0159  ABIR2 as the source-agnostic biosignal and training boundary
    0158  ABIR training storage: explicit resource profiles and versioning
    0074  ABIR production migration
    0075  Training datapath optimization

## Why the order is the whole point

These four are a chain, not a set. 0159 defines where the boundary IS; 0158
defines how storage is shaped across it; 0074 moves production onto it; 0075
optimizes the path through it.

Taken out of order the work is wasted rather than merely early. Optimizing the
datapath (0075) before the boundary settles (0159) tunes a path that then moves.
Migrating production (0074) before storage profiles exist (0158) migrates onto a
shape that has to be re-migrated. Each inversion costs a rewrite, not a rebase.

0075 is the one to watch: it is the most tempting to start, because datapath work
shows immediate numbers, and it is the one whose value evaporates if the boundary
shifts under it.

## Out of scope

Codec quality, latent scaling and the paper — those are OW-WAR-0053.
Repository topology and the boundary ratchets — OW-WAR-0051. If a step here
seems to require moving a crate between repositories, it is 0051's problem and
almost certainly the cross-repo error 0051's intent warns about.
