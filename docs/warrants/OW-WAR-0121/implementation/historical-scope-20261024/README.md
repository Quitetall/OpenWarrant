# W121 historical scope correction

The earlier audit treated today's journal diagnostic names as a discrepancy
requiring W121 to be amended. That conclusion did not distinguish the delivered
version from a later FORMAT-1.0 change. It is corrected here; original signed
records and current-source observations remain unchanged.

The original namespace is observed in a clean historical source at
`d21710454011b204a69c7a2f6364431bb4a55775`. Registered `53-storage.sh`, through
its public battery helpers, passes 23 controls, zero failures, using sealed debug
and release binaries rebuilt from that exact source. The positive corpus,
crash/prestate/symlink controls, torn-tail/malformed distinctions, manual repair,
six-writer inventory and planted direct-write refusal pass. The release control
also observes no fault-hook string in its exact binary.

The atomic module and registered plant match W121's D-001 and D-011 recorded
bytes. D-008's actual recorded digest is found at source
`5253cb8500ea66cdd89c27f995f8562146a2540b`; that version contains
`journal.torn-tail`, not `work.journal-torn`. The renamed diagnostics are
introduced by descendant `2806927332e23932192ba7940346c9d89051a626`
(FORMAT-1.0 work tracking). No current code is changed to restore old names.

The isolated replay preserves source timestamps from the byte-identical clean
historical source for the plant's release-freshness predicate. Binary timestamps
were not changed. Tool identities and source setup are retained. This is a
historical performer observation, not a current-release qualification.

The current-source CI observations still prove their stated newer behavior.
That CI job's release-hook UNKNOWN remains UNKNOWN; separate historical results
are not relabeled as that job's result. Versioned evidence supports implementation
completion without treating a later rename as an original implementation gap.

Implementation is reported complete, unverified. Current delivery/provenance
reconciliation, independent controlled assessment and human acceptance remain
separate. No signed atom, authority, disposition, resolution, pin or ADR was
changed. Other moving-source delivery gaps are not cleared by this observation.
