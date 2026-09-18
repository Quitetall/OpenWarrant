# Runtime preservation: remaining integration boundary

Source inspected at OpenWarrant `376624a` and Knowledge Fabric `bcb7accc`.

## Available records

`openwarrant-core/src/seam.rs::KatanaReceipt` defines the provider receipt and
checks its required fields and expected dispatch digest. Its receipt digest and
runtime event-log head are provider-owned facts, not a local gate receipt seal.
`openwarrant-cli/src/resolve.rs::runtime_receipts_match_the_basis` still documents
that there is no local receipt store. Its result must not be used to assert that
runtime records do not exist in the provider.

Knowledge Fabric maps `warrant-runtime-receipts` to
`work.warrant_runtime_receipt` in its preservation import target inventory.
The preservation test exports and restores those rows together with Warrant
contracts, dispatches and action history. Its nested receipt bodies are synthetic
fixtures. They demonstrate preservation of the populated provider category;
they do not demonstrate a real Katana execution.

## Remaining work

1. Supply a provider export/resolver boundary for local non-human stage archives.
   Preserve exact receipt bytes, provider identity, Warrant/contract/stage and
   dispatch binding, and the provider snapshot identity. Do not copy a provider
   boolean into local completeness or fabricate a replacement receipt store.
2. Check both missing and mismatched bindings offline, retain all selected
   historical attempts, and distinguish no attempt from unavailable history.
3. Exercise a real no-paid-call runtime record through the provider and archive
   path, preserving its actual provenance. Existing synthetic records remain
   labeled as fixtures.

Until that path exists, `preservation/context.rs` correctly leaves non-human
runtime coverage unavailable. Source-complete human-stage fixtures and actual
OW30 selected-local roundtrips do not settle this integration gap. No change to
legacy resolution, human authorization or assurance status is made by this note.

## Provider reader progress

KF draft PR #5 now implements authenticated offline receipt/dispatch/contract
selection and optional exact stage packet binding. See `provider-stage-bindings.md`
for source identity, tests and the observed CLI journal side effect. Local archive
assembly has not yet consumed this provider reader, so its unavailable runtime
coverage remains unchanged.

## Missing-receipt inventory

KF candidate `dffe06858d8d60e753f1496dadf56eba4799d867` adds
`dispatchesWithoutReceipts` to the authenticated offline projection. This lists
selected provider dispatches with no retained receipt row, separately from
`unmappedDispatchDigests`. A failed receipt still counts as retained evidence;
absence is not an assertion that a worker never ran. Neither list establishes
complete stage coverage or permits changing local archive coverage to retained.

The new signed-package fixture retains two dispatches, a failed receipt for one,
and reports only the other as lacking receipt evidence. Five reader tests and
all 43 export-package tests pass after rebuilding the CLI; TypeScript build and
focused ESLint pass. An initial CLI parity run used stale compiled output and
failed; the rebuilt full suite supplies the passing result. Retained suite log:
`provider-receipt-gap-tests.log.gz`. Provider PR5 carries this change; archive
assembly, real runtime proof and formal qualification remain open.

## Native broker observation, no model execution

Installed Katana 0.4.0 served its real MCP broker in an isolated temporary workspace
under reader policy with bubblewrap configured. Reading the fixture file succeeded;
writing a new file returned E_DENIED and created no file. The exact nine-event
provider log and MCP protocol are retained in `katana-native-broker.jsonl.gz` and
`katana-native-protocol.json.gz`; binary/log digests and bounds are recorded in
`katana-native-broker-observation.json`. No model event occurred. The configured
provider endpoint was loopback port 1; no model tool was called.

`katana sessions --cwd` did not enumerate this MCP session: the native MCP path
uses the separate sessions/mcp store. Its session.created event confirms the exact
fixture workspace. `katana replay` exited 2 with no main model.request; this is
not a successful replay or an execution failure. This observation proves native
broker read/refusal logging only. It has no Warrant dispatch binding or PromptIR
and cannot be relabeled as an OpenWarrant KatanaReceipt. Provider archive ingestion
still needs the real bound stage receipt rather than a fabricated wrapper.

Provider input refusal correction: `0b95d685c4c371f88af7a5754fc3e3929bbef8b4`
opens bounded files with O_NONBLOCK as well as O_NOFOLLOW. Before the fix a FIFO
dispatch argument timed out because open blocked before the regular-file check.
The built CLI now exits 1 with no stdout and a non-regular-file diagnostic. The
43-test export suite, TypeScript package build and focused ESLint passed. Retained
red/green logs prove the concrete failure and refusal; latest hosted checks remain
separate. No receipt or authority claim changes.
