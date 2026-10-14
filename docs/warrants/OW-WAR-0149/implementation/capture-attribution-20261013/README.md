# Capture attribution — performer observations

Source basis: main 7b962476. This is an OW149 prototype, not an accepted standard,
provider seal, permission, independent verdict or operator qualification.

The new pure SDK module `runtime_collector::attribution` prepares and authenticates
a bounded canonical proof of the current collector key's claim over exact payload
bytes and their recorded dispatch. The host must supply trusted current authority,
an authenticated enrollment, checked current binding, actual payload and a real
signature adapter. It does not authenticate those host inputs by naming them.

The proof binds collector, current authority, enrollment, Warrant, contract,
dispatch, stage, attempt and local payload digest. This digest is not a provider
receipt digest. A signature authenticates the key holder's statement, not the
truth of execution, expense, confinement or collection timestamps. The authenticated
object has no public constructor or wire decoder. It grants no execution rights,
human review or Warrant assurance. Ordinary unsigned captures remain unverified;
no existing store, active authority or record was changed.

The exact wire is JCS under `oh.war/runtime-capture-attribution/v1-draft.1`, with
OpenSSH namespace `openwarrant-runtime-capture-v1`. Wire is at most 64 KiB, a
signature at most 16 KiB, and actual payload bytes at most 6 MiB. Supported malformed,
extra-field, duplicate-field and noncanonical input refuses. Unsupported schema is
UNKNOWN. Authority changes, scope and payload/binding substitutions refuse.

Three new SDK controls and eight enrollment regressions pass. The reference adapter
accepts only its enrollment and capture namespaces; real OpenSSH controls prove
exact bytes, wrong key/changed payload refusal and cross-namespace replay refusal.
The full SDK-to-OpenSSH fixture passes with private test keys only in memory. Its
synthetic human label is not a human approval. Six signature controls pass; two
provisioned namespace fixtures remain ignored and supply no new operator evidence.
Core all-target/all-feature tests and core/CLI all-target/all-feature Clippy pass.

The first run failed because the module was absent. The first implementation
compile failed on an unsupported hexadecimal formatting trait; explicit digest
bytes corrected it. Both observations remain retained. Format and broader CLI
checks are recorded after final source staging.

Remaining: native/import transport wiring, protected caller and source custody,
revocation fencing, real provider/workload observations and required human/independent
qualification. The original t-cba8f import/enrollment items and OW149 remain open.

Final staged CLI suite: 475 library, 3 binary and 279 integration tests PASS;
zero failures, four existing ignores. Core all-target/all-feature aggregate:
743 PASS, zero failures. These are local performer observations, not an
independent disposition. Full hosted CI remains required before merge.
