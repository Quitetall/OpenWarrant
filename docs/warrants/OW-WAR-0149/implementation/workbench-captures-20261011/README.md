# Workbench capture attachment and automatic worker return

Unverified performer observations. No independent verdict, native qualification
or human acceptance. Implementation source:
`b167f6c3d68b5da7d947d2613559b4bbaa3138d3`.

Public seams under test: authenticated HTTP run/capture endpoints and the
configured harness's process request/result interface, using the actual SDK CLI.
The operator's existing exact Dispatch link selects the corpus subject; clients
cannot select another subject or native policy. Import retains original bytes in
the core store. Immutable local pointers bind the actual workbench attempt,
source digest, runtime Dispatch and SDK capture digest. Replay survives restart.

The first attachment test reached the absent route and failed with HTTP 404.
Its first implementation run lost its connection during initial draft creation,
before attachment. An instrumented service-stop run retained stderr and passed
attachment/restart/replay. A separate refusal control used another actual
compiled Dispatch: it was usable through the core capture endpoint but refused
for the workbench attempt. Extra Warrant, native-store and qualification fields
were refused without attachment.

The first two worker-mode tests and first implementation test lost connections
or timed out during initial draft creation. They did not reach the new result
handler; they are unavailable observations, not observed protocol refusals.
After implementation, the three targeted capture tests passed together (21.660
seconds): automatic worker return, HTTP attachment/replay and identity refusals.
A further test passed (39.111 seconds) for a worker returning a receipt for a
different actual Dispatch: implementation completed unverified, the optional
capture remained refused, no attachment existed and native assessment remained
unknown. An earlier test-construction command had a quoting syntax error; the
resulting missing-test error is retained separately, not called a refusal control.

The optional draft v2 harness result adds only runtime_capture to the v1 fields.
Automatic imports respect the remaining execution deadline and retain a compact
summary. Invalid/unavailable evidence stays refused/unknown and cannot clear an
assurance gate. Ordinary implementation checks remain required. Capture writes
serialize separately; publication failure reports partial retention rather than
claiming absence or success. Local attachment limits are 16 KiB per entry, 256
per attempt and 1024 overall. No authority or native verification policy is
activated by these routes.

Full-suite/hosted checks remain separate gates. Actual provider seals, production
custody/spend/confinement, changed-workspace context, independent qualification
and real study participants remain open. OW149 is in progress; OW95 has no
participants. Synthetic receipts remain labeled and unqualified.

The complete managed web run at `ed959842` finished 227 tests in 468.168
seconds with one failure and one error (exit status 1). All four new capture
tests passed. The failures were a board response of HTTP 503 (SDK unavailable
or timed out) and a connection lost while creating the initial draft for the
concurrent-edit test. The full suite did not pass. A focused rerun finished two
tests with one server-startup-unavailable failure; assertions and timeouts were
unchanged. Hosted checks remain required; unavailable local observations are
not promoted to a full pass.

After merging the updated Dispatch prerequisite and main at `4fa747b3`, the
four targeted tests finished in 81.163 seconds: three passed and one timed out
during initial draft creation, before capture attachment. This combined-source
run is incomplete. Strict Clippy and 13 Dispatch CLI integration tests passed
on the merged prerequisite. Generated record checking timed out locally;
compilation completed with zero errors. Hosted gates remain required.
