# OW-WAR-0080 implementation

Unverified, in progress. LAMU projection provider implements task and master views,
four roles, exact binding slices, explicit source labels, conservative summary
provenance, stage overrides, sorted diagnostics and bounded construction.
The attached observation exercises the actual pinned provider. Twenty provider
public tests and Clippy passed; independent specification and standards reviews
passed after stage, provenance and aggregate allocation repairs.

Task readiness remains explicitly blocked pending record evaluation. Offline
package verification belongs to 0081. No human signature or assurance is claimed.

LAMU commit review findings about full-source access and full-blob accounting
were rejected after checking F7/T37: all parsed routing sources and selected blobs
must be supplied in full, with access checked before delivery. Slice-only accounting
would undercount the emitted package. The reported formatting issue did not match
the formatted source.

Next: offline package semantic verification, cache behavior, integration exits.
