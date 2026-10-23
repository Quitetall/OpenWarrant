# Projection observations

`run.py --provider-root <LAMU-Rust-workspace> --output <new-file>` checks the
shared profile and runs the three original provider projection tests. It records
the exact provider/SDK revisions, source hashes, commands and output.

`check_observations.py --war <SDK-CLI> --transcript <probe-stdout>
--record-dir <new-directory>` checks the additional external probe's retained
responses through the SDK. The original test caller and its execution identity
are kept with the W80 evidence. It runs only in a disposable optional-provider
checkout; it is not an OpenWarrant dependency or a provider implementation patch.

The probe emits actual task/master results and named refusals. The consumer
checks canonical packet bytes with the existing SDK decoder, source descriptors,
exact unit slices, common subject membership across roles, summary provenance,
and unchanged snapshot output after the live input changes. Role instructions
and stop conditions are checked against F6. No model is invoked for projection.

The synthetic performer claim and authority labels are refusal fixtures, not
real approvals. Decoding and retained caller observations do not establish
producer trust, independent qualification, permission or human acceptance.
