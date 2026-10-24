# W144 retention audit

**Implemented test improvements; unverified.** The current candidate tool passes
seven feature controls plus the positive corpus check (8/0). The
[observation](observation.json) pins source hashes, tool identity and limits;
[actual output](current-tool-controls.log) records each check.

The earlier collision control computed a directory snapshot but never compared
it after refusal. It proved the old authorization survived, not that every
Warrant file stayed unchanged. The plant now compares regular-file names and
bytes below the fixture Warrant. A separate third revision supplies an existing
identical archive and checks that authorization succeeds while the archive's
bytes and file identity survive. Revision 2 still tests automatic archive creation.
The original attestation/archive-removal refusal and active-record checks remain.

Two deliberate fixture mutations prove the new assertions detect their targets:
adding another Warrant file after collision refusal, and replacing an archive
with an identical copy. The [mutation run](test-mutations.log) has exactly those
two failures (6 pass, 2 fail). These are mutations of the test fixture, not
observed defects in production authorization code.

The first harness run had seven passing feature controls and a failed positive
corpus check: the changed plant had not been reflected in the private clone's
generated progress. Its five drift diagnostics and initial run remain in the
retained evidence directory. Public compilation corrected that private fixture;
no signed source, delivery digest or resolved pin was changed. A fresh current
branch build then passed all eight checks. The build identity marks its dirty
source honestly; protected CI must still assess the committed candidate.

The [resolution dry run](closure.json) reports six passes and eight UNKNOWNs,
including current artifact digests, independent obligations, admissible gates,
unknowns, blockers, judgments, independence and risk authority. Earlier D-001
ownership by W148 and original delivery records remain untouched. W144 remains
in progress; the broad signing ceremony and independent acceptance are not
established by these performer observations.

Fixture keys and agents were disposable. The owned scratch clone and test state
were removed after execution; durable logs, source patch and binary are retained
under `/mnt/4tb/artifacts/OpenWarrant/keep/ow144-retention-20261024/`.
