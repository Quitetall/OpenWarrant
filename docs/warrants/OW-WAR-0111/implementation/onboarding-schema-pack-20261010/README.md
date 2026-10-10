# Checked schema sources during setup

Unverified continuation of OW-WAR-0111, addressing the schema identity gap observed
in the actual native preservation prototype. Fresh `war init` now retains the
binary's schema pack and its exact member files under `schemas/`. The same setup
path serves plain, preset and program initialization. No generator feature,
network call, signature or manual schema copying is required.

The bundled pack's identity, version, canonical bytes, member digests and
transitive digest are checked before filesystem mutation. Existing plain files
with exactly matching bytes are preserved, including their read-only permissions
and modification time. Different files and links refuse before init creates a Git
repository or configuration. Files are created without overwrite; incomplete
writes remain visible as a setup error. This is setup in the caller's trusted
workspace, not a sandbox or an atomic fence against another privileged writer.

The schema version and all committed schema bytes remain unchanged. Existing
initialized repositories are still refused by init; this does not silently
upgrade their configuration or declarations. An archive's other missing inputs
still block whole import. Installing schemas does not establish native provider
trust, historical IR, independent verification, human acceptance or assurance.

## Public observations

The existing archive integration test previously supplied schemas by copying
files manually after init. Removing that workaround produced the expected failing
assertion: schema/compiler identity was not retained. The repaired public setup
and export now retain the pack; source-detached inspection succeeds, and altered
schema bytes still refuse with `schema member digest mismatch`.

An old-binary control accepted setup with an unrelated owner pack and created its
configuration. The repaired public init refuses conflicting pack/member bytes,
non-directory schema paths and directory/member links before writing config,
Git state or other schema members. A partial, matching, read-only pack survives
unchanged while the missing members are installed and successfully archived.

Rust toolchain 1.97.1. Logs and the old-binary control are retained beside this
file when terminal checks complete. Full hosted gate and formal qualification
remain separate; OW111 remains in progress.

## Terminal validation

- Schema preservation regression: failed before repair, then passed after repair.
- Public setup conflict/read-only controls: two tests passed (five refusal plants).
- CLI library/binary tests: 445 and 3 passed.
- First concurrent integration run: 213 passed, two failed, four ignored. The
  progress comparison ran against stale generated data after new documentation;
  the drafting test exceeded its five-second startup wait under concurrent load.
- Concurrency control alone passed without changing its deadline or assertion.
  A first projection rerun started before regeneration completed and failed;
  the subsequent current projection comparison had zero differences.
- Final integration run after completed regeneration, with four test workers:
  215 passed, zero failed, four explicitly ignored, 224.46 seconds.
- All-target/all-feature CLI Clippy with warnings denied: passed, 52.61 seconds.
- Formatting and whitespace checks: passed.
- Generated-record check: 2028 pass, 625 warnings, zero errors or unknowns.

Initial failures remain in the compressed logs. The ignored native runtime tests
and hosted whole-repository gate are not counted as local passes. OW111 still
needs the native runtime preservation resolver, historical reconstruction,
actual KF native roundtrip and formal qualification.
