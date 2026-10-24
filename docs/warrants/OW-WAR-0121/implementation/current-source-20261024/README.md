# W121 current storage audit

All eleven source/document/plant files checked here are byte-identical to
protected CI head `1f0fa2e34ef608073ac439653dd723ac9609c7b1`, run 40033704288.
The retained excerpt observes crash preservation during signing and compilation,
stray-temporary-file reporting, prestate-moved refusal with unchanged-prestate
controls, signing/view symlink refusals, torn-tail offset and manual repair,
malformed-line distinction, no automatic truncation, six-writer inventory and
its planted-direct-write refusal. These are bounded Linux observations.

The CI run reports release fault-hook absence UNKNOWN, not pass. The earlier
`storage-controls-20261012/release-observation.json` records a successful clean
release build and a release-equipped plant run at its exact artifact. The
complete debug/release fault-hook region is unchanged from that source. This
supports the unchanged compilation guard without pretending the older runtime
observation is a new current release execution.

## Diagnostic versions — corrected interpretation

OBL-004 names `journal.torn-tail` and `journal.malformed`. Today's tool and
plant report `work.journal-torn` and `work.journal-malformed`. The historical
replay and recorded journal pin confirm the original names before the later
FORMAT-1.0 rename. The previous recommendation to amend W121 solely for this
rename was too strong; see [the historical scope correction](../historical-scope-20261024/README.md).

The observations above remain specific to current-source CI. Implementation is
reported complete, unverified, with versioned evidence for the original behavior.
No authored signed atom, deliverable pin, authority record, independent disposition
or resolution was edited. Existing closure gaps remain separate.
