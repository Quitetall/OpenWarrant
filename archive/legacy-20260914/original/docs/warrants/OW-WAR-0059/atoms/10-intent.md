---
schema: oh.war/atom/v1
warrant_uuid: 01a060c6-64a5-71e1-b994-133d3c1e19d2
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Make "0 satisfied" a number that can change.

## Where the corpus stood

After OW-WAR-0055–0058 and the owner's signatures, thirty-eight Warrants had
twelve of §56.1's thirteen requirements met and the same one unmet: "every
required gate has admissible result". Not because no gate had run — `cargo
xtask gate` runs on every push — but because nothing a fresh clone could read
said so. Receipts were written under `docs/receipts/`, which is gitignored on
purpose (a receipt carries wall-clock times; committing one as a side effect
of running would dirty the tree on every run), and the corpus projection was
built from tracked inputs only, so it read requirement 5 as unmet for every
Warrant and said why in a caveat.

And when all thirteen were met, nothing could record it. `war resolve`
without `--dry-run` printed a refusal: §56.2's record needs a resolver, an
acting role and a stated meaning, and the command had no authority model. The
authority model has existed since `war authorize`; the resolution seam did
not.

## What this delivers

Two things, in the order the SAS puts them.

**A receipt becomes evidence by being recorded FOR a Warrant.** `war evidence
record <alias>` runs the gates the Warrant's assurance atom cites and mints
each §44.6 receipt directly into `docs/warrants/<alias>/gate-runs/`, with
`subject_digests` naming the Warrant's current contract digest. Requirement 5
reads that directory and nothing else, so `war status` and `war resolve`
answer from the same tracked inputs. A run counts only when its receipt
reseals, agrees with the run, and is bound to the contract as it compiles
now; a receipt for an earlier revision is a record of something that
happened and is not evidence about this one.

**A resolution is a human's, through the third two-half seam.** `war resolve
<alias>` emits a request naming what a signature would bind and which
outcomes §38.6 permits; `--response` ingests a resolver's signature through
the authority register, refuses every agent, refuses `satisfied` over
unestablished obligations, refuses a moved contract, and writes
`resolution.toml` once. The record binds the contract digest, the assurance
snapshot digest and the artifact manifest digest that were only ever
declared fields before.

## What this does not do

It does not resolve anything. It runs two real gate runs and emits two
requests; whether a human signs them is the human's. It records evidence for
the Warrants whose obligations are established AND whose assurance atom cites
a gate — OW-WAR-0010 and OW-WAR-0020. The third with established obligations,
OW-WAR-0016, cites no gate at all, so there is no required result to record;
that is an amendment somebody authorizes, not a receipt for a gate nobody
asked for. The thirty-five whose resolution would honestly read
`not_satisfied` get no receipt either: it would change nothing they could
truthfully close as.
