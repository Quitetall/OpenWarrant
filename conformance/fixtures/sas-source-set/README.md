# Source-set acceptance subject vector

This ASCII-only vector exercises the 0074 AM-001 domain through the public CLI
in `crates/openwarrant-cli/tests/sas_source_set.rs`. That fixture provides the
source bytes; `manifest.json` is the exact candidate manifest including its final
LF. `subject.json` is canonical JSON plus a display LF; the display LF is excluded
from the digest preimage.

The independent expected digest was calculated with Python's `hashlib.sha256`
over UTF-8 `oh.war/sas-source-set-subject/v1`, one NUL byte, and the subject's
canonical JSON bytes. Sorting keys and using compact JSON produces RFC 8785
bytes for this vector because every key/value is ASCII and there are no numbers
in the subject. This is a fixed input/output vector, not a replacement canonicalizer.
