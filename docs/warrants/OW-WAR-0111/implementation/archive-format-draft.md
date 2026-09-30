# Experimental preservation transport — proposed, not adopted

OW111 prototypes a distinct `oh.war/preservation-archive/v1-draft.1` transport.
It does not alter `PortableExport`, `war_export_digest`, or existing contract IR.
Promotion to stable format requires an explicit accepted format decision.

One canonical RFC 8785 JSON object contains schema, subject, producer identity,
records sorted by path, coverage for every required legacy export category, and
optional namespaced extensions. Each record has a portable relative path,
SHA-256 content digest and either canonical base64 bytes or an external marker.
Coverage records retain path references, or explicitly mark a category absent
with a reason, or unavailable with a reason. Unavailable is never complete.
Known absence is a recorded observation, not a synthesized placeholder for a
populated category; source assembly must prove it. Codec cannot infer source truth.

Archive identity is `sha256:` plus SHA-256 of canonical JSON preimage
`{"digest_domain":"oh.war/preservation-archive/v1-draft.1","payload":ARCHIVE}`.
The digest is returned separately, avoiding self-reference. Evidence file digests
are SHA-256 over exact bytes. No existing digest function/domain is repurposed.

Decoder requires canonical input bytes. This also refuses duplicate JSON keys,
unknown fields, reordered/noncanonical arrays and noncanonical base64 rather
than silently normalizing untrusted input. Unknown optional data belongs in
namespaced extensions and participates in digest identity. Versions fail closed.
Portable paths reject absolute/traversal/backslash/colon/control/Windows-reserved
components, case-fold collisions, and file/directory prefix collisions.

Limits apply before JSON decoding and during content validation: input bytes,
record count and aggregate embedded bytes. Referenced bytes are not considered
reconnected until a resolver returns bytes and their size/digest is checked.
Codec has no file writes, signatures, authority activation or completion marks.
Filesystem safety, actual category assembly, IR reconstruction and KF import are
subsequent stages; codec success is not complete preservation qualification.
