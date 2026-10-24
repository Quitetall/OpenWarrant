# W124 adoption implementation audit

The guide previously led directly into signer files and SSH approval. It now
starts with routine ticket tracking, states that signing setup is optional for
that path, and labels the remaining governed setup. Its description of pins
also distinguishes every declared delivery from `--resolved-only` pins.
Only the guide was changed; no authority step or implementation was edited.
Its new bytes were recorded through public `war deliver` for D-007 only.

## Observed controls

The public battery helpers ran the registered `59-adoption.sh`,
`75-init-program.sh` and `99-init.sh`: 40 passed, zero failed, including the
initial whole-corpus positive check. Together they cover the bounded baseline,
namespace-aware telemetry, no retroactive ownership/assurance, ADR-pointer-only
behavior, unchanged historical telemetry, authority-region preservation and
negative control, outside/inside Git setup, missing-Git UNKNOWN and unchanged
scripted setup. No reader, schema or signed atom was changed.

The named guided tests (baseline present/skipped, existing authority never
rewritten, agent remains performer) passed in protected CI run 40033704288 at
`1f0fa2e34ef608073ac439653dd723ac9609c7b1`. All nine relevant source/template/
plant files are byte-identical to both that source and the sealed tool source.
The unit tests were not rerun as a new local Cargo build.

The new routine path was also exercised in a three-commit repository: init,
create, prime, claim, done and prime all returned zero with no reachable SSH
agent. The vibe preset says `signing = "never"`; no signer files,
authorizations, resolutions or attestations were created. Its task was to
observe these records, not implement a software feature. The complete fixture
and raw command streams remain under the named durable artifact directory.
This is not a consenting-developer usability study or assurance qualification.

## Preserved failed runs

The first focused run exited 9 on its setup guard: TMPDIR was inside the
worktree, so a supposed outside-Git directory was already inside Git. The
unchanged tests were rerun with an external temporary directory.

That run observed 39 feature passes, but its initial corpus check failed.
The retained full check named only two stale progress views; those views were
regenerated through the public compiler. After committing the intended source
and regenerated views, the same run passed all 40 checks. Neither failed run
is called a pass. No test was deleted, weakened or hand-edited.

Full raw evidence and the fixture archive are retained at
`/mnt/4tb/artifacts/OpenWarrant/keep/ow124-adoption-20261024/`.
Implementation is reported complete, unverified. Independent controlled review,
current provenance reconciliation and human acceptance remain separate. The
known mutable-baseline limitation remains documented; this audit neither
changes nor newly accepts that risk.
