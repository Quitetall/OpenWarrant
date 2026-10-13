# Attributed capture import — performer observations

Source basis: 7852bbe9 (PR265, pending hosted CI at implementation start).
Ticket t-af9b7 implements one OW149 increment. Parent t-cba8f/i-1133 and OW149
remain open. This is candidate infrastructure, not an accepted protocol, an
independent disposition, a real operator enrollment or a release qualification.

The new explicit import signs a canonical envelope containing the entire capture
request and original receipt bytes. The checked recorded Dispatch supplies the
Warrant/contract/stage/attempt binding. A trusted host authenticator checks the
collector proof before provider assessment and again before retention. It cannot
return an authenticated object with a different subject. Host settings supply
current protected authority, active enrollment, repository/collector and actual
execution UID; protected executable bytes must match the enrolled digest. The
sealed image is retained while authenticating and is not executed by this import.
Authority/configuration and active enrollment are rechecked around signature
verification. These comparisons are not atomic revocation or writer fencing.

The added SDK import control uses real in-memory software Ed25519 signatures and
OpenSSH verification with synthetic authority. It checks successful attribution,
changed metadata, changed receipt, late host refusal, prior-record preservation,
unsigned compatibility and bounded/unsupported inputs. The first compile lacked
ProviderFailure::Unsupported handling; the corrected code reports it UNKNOWN.
All unsuccessful compile observations are retained, not reported as test passes.

The protected_namespace_roundtrip fixture was explicitly run through a disposable
user/mount namespace, chroot and temporary filesystem. Actual namespace UID 0
creates fixture authority/activation; UID 1 cannot modify that authority and uses
the attributed CLI command. The command imports the exact signed capture and
refuses changed metadata. Existing controls cover inactive/stale/revoked enrollment,
wrong provider/UID, changed protected executable and native refusal. A temporary
Rust borrow failed compilation first; the fixed run executed one wrapper test and
passed. Child observation transcripts are in namespace-2.log.gz. Private fixture
key bytes remain in memory only. This is software fixture authority, not a human act
or operator custody qualification. No actual provider/model call was made.

Retained records include signature/proof but explicitly set trusted_on_read=false
and assurance_granted=false. Native execution/seal verification remains a separate
provider check; declared collection times, identities, command and status do not
become facts merely because a key holder signs them. Ordinary unsigned imports
retain their interface; existing records are not rewritten. Broader native/provider/operator, independent
verification, authority fencing, policy and human qualification remain open.

First broader suite: 475 library and 3 binary PASS. Integration: 279 PASS,
one generated corpus parity failure and four existing ignores. The new ticket
had not yet been compiled into the projection. Authored records are staged and
compiled before the integration replay; the checker is unchanged. CLI all-target,
all-feature Clippy with warnings denied passed.

Generated parity passed after compilation. That replay found a different, initial five-second draft checkpoint timeout (document_draft.rs:167). Exact replay passed in 0.58 s; the unchanged complete integration replay passed 280 tests, zero failures, four existing ignores. Cause remains UNKNOWN and is tracked in t-aade8. No expectation or deadline was weakened. Both failed suite logs and successful replays remain retained. Final source checks: 475 library, 3 binary and 280 integration PASS; explicit protected namespace wrapper PASS; all-target/all-feature strict Clippy PASS. Hosted CI remains required before merge.
