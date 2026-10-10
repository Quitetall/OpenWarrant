# Current source for direct workflow dependencies

Unverified performer implementation under the Phase 3 reference workflow
(OW-WAR-0090). Parent source: `315f3494ce25a39246a2db6d9befbb5d2fdb9188`.
No human act or independent disposition is recorded.

The old start evaluator matched a dependency's retained completed result to the
startup configuration, without reading its current saved draft. A public HTTP
control completes a dependency through the configured synthetic harness, prepares
a dependent Warrant, then edits the dependency through the real SDK-backed draft
endpoint. The old preview still reports ready (`red-execution.log`, 0.557 seconds).
The verifier HTTP control first prepares a valid job, changes the dependency
source, then requests a distinct job; the old service creates that job with HTTP200
(`red-verifier.log`, 0.517 seconds). A narrower snapshot control also reproduces it.

Preview and execution now read each direct dependency's current source digest
under the existing authoring/execution lock and require it to match the configured
policy. Verifier snapshots enforce the same current-source requirement before
preparation and subsequent controller actions. Existing exact-result, policy and
writer checks remain in place. Known source changes refuse; unavailable source
observations retain the service's existing unavailable handling. No configuration,
source document, historical result or prepared job is rewritten by these reads.

The repaired public execution control observes a blocked preview and HTTP409
start, no additional attempt, and identical historical source and completion.
The repaired verifier control requires successful preparation before the change,
then HTTP409 without a new job and with the prior job preserved.

## Validation

Rust 1.97.1 own CLI built successfully; no Rust source changed. Focused repaired
HTTP execution PASS, 0.718 seconds. Verifier snapshot, service and scheduler suite
PASS, 17 tests in 6.929 seconds. Python syntax checks PASS. Full web-workflow suite PASS, 215 tests in 214.695
seconds (`web-suite.log`). Final generated-record checks are retained in
`record-check-summary.json`; record checks are not runtime qualification.

Early HTTP attempts failed to reach the control because of startup/socket
timeouts while the host was under heavy memory/I/O pressure. One initial test
also used a nonexistent fixture helper; that helper was corrected. These failures
are retained in raw observations and are not reported as bug reproduction or
passes. The final red controls reach the intended source-currency assertion.

## Bounds

This covers direct dependencies in the opt-in SDK-draft reference workflow. It
is not native legacy StageDispatch integration, transitive invalidation, protected
operator deployment, provider verification or atomic fencing outside the service
lock. Retained historical reports are not rewritten. Synthetic harness runs prove
the tested protocol, not real-agent quality or human usability. OW90 and OW95
remain incomplete; the three-developer study is still pending.
