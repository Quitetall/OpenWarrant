# Schema and producer identity capture

Archive export now records the observed executable SHA-256, CLI package version,
and schema-pack identity in `__ow_archive__/producer.json`. This fingerprint is
an observation, not authentication or permission. No executable is imported or run
from the archive, and the fingerprint does not attest source-to-binary provenance.

When the repository has `schemas/pack.json`, export retains its exact bytes and
all declared JSON Schema member bytes. It checks the supported pack identity and
version, safe member names, each member SHA-256 and the pack's transitive digest.
The pack must be canonical JSON, with the generator's optional final newline;
duplicate keys cannot silently normalize into a different pack. Reads use the
existing no-follow bounded source reader and archive aggregate/count limits.

Inspection and reconstructed import/re-export recheck members against the retained
pack, so changing a member and its archive record hash alone is insufficient.
A pack without its producer record refuses. If the repository has no pack, export
retains the producer observation and explicitly leaves schema coverage unavailable.
This does not silently generate a replacement for the repository's pack.

A source-detached test checks retained pack membership and actual executable
fingerprint, then reconstructs after both source docs and schema directory disappear.
Tampered source-schema bytes refuse export before creating output; tampered
archived-schema bytes refuse inspection even when their outer record hash is updated.

This is preservation of the declared pack and observed tool identity. It does not
validate every corpus record against JSON Schema, prove all possible schema families
exist, authenticate the compiler, or establish other missing archive categories.
