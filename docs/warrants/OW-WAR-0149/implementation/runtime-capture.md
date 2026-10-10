# Dispatch-bound runtime capture

The candidate BLUT process verifier is documented in
[BLUT process adapter](blut-process-20261009/README.md). It requires explicit
host-trusted configuration. Synthetic public transport tests do not establish
native seal validation or provider qualification; default CLI enrollment remains
unimplemented.

Candidate reference SDK and CLI implementation, 2026-10-09. Performer-reported work only; no native provider qualification, independent verdict, participant acceptance or human signature.

## Use

A collector supplies one repository-relative JSON request. The agent or operator runs:

```sh
war runtime import OW-WAR-0149 --request capture-request.json --json
war runtime show OW-WAR-0149 sha256:<returned-content-digest> --json
war runtime assess OW-WAR-0149 --selection selected-captures.json --json
```

The SDK entry points are `openwarrant_cli::runtime_capture::import` and `show`. This filesystem implementation belongs to the reference CLI library; the core receipt matcher remains pure. The command emits the usual report envelope. Successful retention is separate from native eligibility, which is explicitly UNKNOWN when no native verification adapter is available. `show` inspects retained data and never trusts saved native verdicts.

The command reads `dispatches/<dispatch_id>.json` from the selected Warrant and requires exactly one matching `dispatch.compiled` journal event. It obtains the Warrant, contract, stage and attempt from those source records, checks the existing canonical Dispatch digest, and checks the current captured Compilation Basis. A caller's request cannot substitute those fields. These local records establish source binding, not authentic execution or authority.

## Request

```json
{
  "schema": "oh.war/runtime-capture-request/v1-draft.1",
  "dispatch_id": "<retained-dispatch-id>",
  "receipt": "provider-output/receipt.bin",
  "provider": {
    "kind": "katana",
    "identity": "<provider-build-identity>",
    "version": "<provider-owned-interface-version>"
  },
  "metadata": {
    "observation_id": "<collector-observation-id>",
    "observed_at": "<RFC3339-collector-time>",
    "original_receipt_ref": "<exact-original-native-reference>",
    "binary_identity": null,
    "source_identity": null,
    "transport": "<actual-command-or-transport>",
    "argv": [],
    "exit_code": null,
    "status": "<observed-provider-status>"
  }
}
```

Metadata preserves the collector's declarations. Missing build identity and exit observations remain null; neither means zero, success or an authenticated identity. Native status text is retained, not interpreted as completed execution. The configured performer identifies the collector as an unverified agent; this is not an authenticated human identity.

## Retention and trust

A single content-addressed JSON envelope holds exact receipt, Dispatch and matched compile-event bytes, their local byte identities, collector declarations, a source-binding observation and the separately labeled native observation. Its `oh.war/runtime-capture/v1-draft.1` schema is a candidate local transport/storage envelope, not a provider-native wire format or an assurance record. Pre-result source observations use a separate draft schema: the existing RC.2 assurance Record requires a result digest, and capture must not invent one. The embedded import-source hash identifies `runtime_capture.rs`, not the complete binary or source closure.

The provider retains ownership of native seals and verification. A trusted SDK caller may supply a `ReceiptVerifier` and dispatch-resolved capability, confinement, registry and spend facts; wrong native bindings refuse before publication. The CLI currently has no configured native adapter and retains bound source data as unverified. Unknown cost remains unknown. Stored native observations must be verified again against the current recorded attempt and policy before eligibility; file checksums and saved PASS-like values do not provide that trust.

Receipt bytes are limited to 4 MiB, request metadata to 16 KiB, retained envelopes to 16 MiB, and source JSON uses duplicate-member/node/depth controls. Reads refuse traversal and link following; nonregular sources cannot block reads. Publication reuses descriptor-relative, no-overwrite, complete-byte retention. Exact replay preserves bytes and the name, including after unrelated journal appends. Changed collector metadata is a different observation. Source loss does not delete the retained bytes; altered retained objects and conflicting occupants are refused. These controls do not isolate a malicious same-UID writer or authenticate declarations.

## Evidence and remaining work

Public CLI and SDK tests cover actual local source capture and synthetic provider facts. No native provider was executed or qualified by these fixtures. `conformance/fixtures/runtime-capture/control.py` independently drives the public CLI for replay, stale basis, collision, alteration and source-loss controls. The code uses existing local SHA-256 byte identities and existing canonical Dispatch hashing; it defines no provider seal or hash domain.

## Current-attempt selection

`assess_selected` is the candidate read-only SDK interface. A caller explicitly selects one retained capture per stage; the SDK obtains the dispatch and compile event from the actual local records. The last appended `dispatch.compiled` event for that stage defines the local compile frontier. A later attempt prevents fallback to a historical success, even if the later attempt has no receipt yet. This does not make the local journal an authenticated execution log or define a scheduler's active worker.

The CLI accepts a repository-relative selection request:

```json
{
  "schema": "oh.war/runtime-selection-request/v1-draft.1",
  "selections": [
    { "stage_id": "STAGE-001", "capture_digest": "sha256:<capture-content-digest>" }
  ]
}
```

Duplicate stage selections, wrong stages, superseded attempts, altered captures and mismatched dispatch/event snapshots refuse. Missing stage evidence and unavailable current sources remain UNKNOWN. Original receipt loss does not prevent reassessment of retained receipt bytes, but missing current dispatch/journal sources prevent establishing current eligibility. Historical `show` remains available separately.

The SDK callback supplies a freshly resolved native verifier and dispatch-specific policy. Saved native verdicts and collector declarations never supply those facts. The CLI currently supplies no native adapter, so selected runtime stages remain UNKNOWN. Native matches establish receipt eligibility only, not assurance. The assessor checks every runtime stage through the existing whole-basis compiler interface and detects observed source/attempt changes before returning. Callers still own locking and execution isolation; rechecks do not provide a transaction or a malicious same-UID sandbox.

Selection is bounded to 256 captures, 32 MiB aggregate selected source bytes, and the existing per-source limits. No record, event, signature or activation is written by assessment.

OW-WAR-0149 and its broader ticket remain open: actual provider-owned interfaces, authenticated collector/build observations, legacy resolver integration, real positive provider runs and independent/participant acceptance are not complete.

## Offline source inspection

`inspect_retained(bytes, alias, digest)` performs the same bounded envelope and embedded blob identity checks as live `show`, without repository or provider I/O. Neither operation trusts saved native verdicts. `war archive runtime-basis` now exposes these retained captures with separately checked Dispatch/journal and reconstructed contract/stage connections. Missing sources and unsupported versions remain gaps; contradictory binding and altered content refuse. See OW-WAR-0111's [capture reconnection observation](../../OW-WAR-0111/implementation/capture-reconnection-20261009/README.md). Source reconnection does not establish a provider seal, current worker, native execution eligibility or assurance.

## Resolution consumer — 2026-10-09

Requirement 12 now uses current-source runtime assessment rather than a constant
refusal for every runtime stage. `resolve::assess_with_runtime` accepts explicit
retained selections and a dispatch-specific trusted native verifier/policy
resolver. It recomputes the same thirteen checks; a matching runtime receipt
clears only requirement 12, not authorization, independent verification or human
acceptance. The capture store is re-read and saved native verdicts are ignored.
Source changes, superseded attempts and conflicting native facts remain unmet.

The reference CLI can inspect a selection with:

```sh
war resolve OW-WAR-0149 --dry-run --runtime-selection selected-captures.json --json
```

This flag is read-only and requires `--dry-run`. It does not configure a native
verifier. Missing native support stays UNKNOWN even if the retained capture
contains a historical matching observation. Normal request, signing and response
ingestion remain fail-closed until a trusted native adapter and its governing
participant agreement are established. No generic command or untrusted JSON
assertion is promoted into a native verifier. No new receipt hash domain or
resolution wire format is introduced.

The public CLI regression was observed failing before this connection: it named
only a generic unmet requirement, with no missing-stage observation. The SDK
controls use a clearly synthetic provider and establish consumer behavior only,
not Katana/BLUT qualification or actual sandbox enforcement.

### Reporting scope

The draft `scope.toml` maps runtime capture, assessment and resolution consumer paths to the existing OBL-001 through OBL-004. It pins the existing repository Bonsai policy and public source revision for report generation. This is a reporting boundary, not authorization, a coverage verdict, native provider qualification or a participant acceptance. The first PR169 report refused because this sidecar was absent; the refusal is retained in external implementation evidence.
