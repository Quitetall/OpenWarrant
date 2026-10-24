# W119: an unasked comparison cannot also pass

**Implementation correction, unverified.** During the completion audit, source
inspection found that `identity.changed` could emit UNKNOWN for an unreadable
Git object stream and then emit PASS because zero differences were counted.
A committed manifest with no UUID had the same problem. Not observing a change
is insufficient when the comparison could not run.

Two public CLI controls reproduce both cases. The [initial run](red.log) has
34 passes and the intended failure. The [expanded run](red-expanded.log) has
35 passes and both intended failures: UNKNOWN is present but PASS is also
present. The correction records whether any comparison was unasked and emits
the clean aggregate PASS only if none was unasked and no identity moved.
It preserves the existing rule names, severities and successful comparisons.

The [corrected run](green.log) passes all 36 identity controls plus the positive
corpus check (37/0). This retains duplicate Warrant/ADR rejection, atom ownership,
changed/new/missing-Git cases, alias severities, manifest version rejection,
BLUT UUID identity, exact warning sets and unchanged ADRs. Nine identity unit
tests and the BLUT stage-identity unit pass; all-target Clippy with warnings
denied, formatting and Bash syntax checks pass. Exact source, patch and tool
hashes are in the [observation](observation.json) and tool-identity.json.
These scoped checks do not stand in for the protected full gate.

The original 23 ADR identities were rechecked against the actual Git blobs at
`5408b60f56c4ccecb5c74407ec8d5e33a9466ef9`. All remain unchanged. The current
corpus has **37** ADR atoms, separately stated in adr-preservation.json; its
six alias warnings and thirteen non-v7 warnings are checked against source
records. The historical 36-ADR observation is retained as history.

The resolution dry run still has eight UNKNOWN requirements and one delivery
ownership warning. No authority, signature, delivery pin, disposition or record
identity was changed. W119 stays in progress pending protected candidate CI,
independent review and the separate human close-out act.

The private clone, fixture state and per-run scratch were removed after the
observations. Durable evidence and the sealed candidate tool remain under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow119-audit-20261024/`.
