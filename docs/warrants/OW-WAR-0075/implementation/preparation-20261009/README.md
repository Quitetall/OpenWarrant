# OW-WAR-0075 delivery preparation

Source 94cf616a; Rust 1.97.1. The current public SDK probe passes all 22 document
cases. A copied valid fixture with a new filename and case ID also passes; adding
trailing bytes to that same source produces exit 1 and FAIL PROBE-DYNAMIC. The
tracked fixture suite is unchanged. Raw stdout, stderr, source bytes, executable
hash and exact commands are retained here. These are performer observations.

Actual delivery admission remains blocked: war deliver OW-WAR-0075 --dry-run
returns deliver.none-declared. This is a missing delivery declaration, not evidence
that the implemented parser is missing. The earlier completed-unverified work
report and original evidence are preserved.

inventory.json compares 67 original source/build/test inventory entries with
current bytes, and identifies 31 proposed delivery targets within the stated
parser/driver/fixture scope plus its notes. proposed-deliverables.toml is a review
proposal only; it is not the active deliverables.toml, a delivered snapshot, or a
grant of ownership. Shared files contain later work too; no current hash attributes
all their bytes to the original OW75 implementation. Original authorship is not
inferred from this capture.

OW-ADR-0021 separates delivered bytes from a signed ownership set. The historical
authorization has no ownership set. Any new ownership grant must use the proper
revision/authorization route; this proposal neither edits the signed contract nor
claims that an old signature covers these paths. Review the exact set before
adoption and record provenance from actual observations, not a guessed producer.

This is one preparation slice, not the actual full-corpus prepare sweep. No
configured paid verifier was called. Independent qualification, secure human
acceptance and legacy resolution remain unestablished.
