# Katana process adapter candidate

Shared contract: OW-WAR-0149 (`01a0f502-4941-70a1-a446-e1eb77dff191`).
This is performer-reported implementation work, not participant acceptance,
independent verification, human acceptance or a completed Warrant.

The reference SDK exports `runtime_capture::katana_process::{KatanaProcessConfig,
KatanaProcessVerifier}`. `for_recorded_dispatch` reads the actual retained
Dispatch and its matching compile event. The host explicitly supplies an absolute
native verifier executable, original event-log path, independently retained
native `b3:` head, scratch directory and bounded timeout/response limit. The
interface is `katana/openwarrant-verification/v1`; no discovery or fallback occurs.
The native implementation is proposed in Katana PR12. There is no Katana crate
dependency or AGPL linkage in the OpenWarrant SDK.

The process receives exact captured receipt bytes and a binding assembled from
those checked records. It verifies the original native log against the supplied
head. The adapter requires its checked native projection to describe those same
receipt bytes, preserves Katana's native head/receipt and PromptIR identities,
and maps terminal completion, failure, halt and cancellation separately.
Capabilities remain the native dispatch permission upper bound. The caller must
resolve the authorized set from the actual task policy, not copy permissions
from the receipt. Cost, spend-cap enforcement and confinement enforcement remain
UNKNOWN. Neither process output nor retained captures grant assurance.

BLUT and Katana share the existing bounded Unix process supervisor: separate
process group, cleared environment, bounded stdout/stderr, strict duplicate-key
JSON decoding, consistent status/exit code, private no-overwrite scratch and
cleanup. Native provider mapping remains separate. The supervisor does not
sandbox a malicious verifier or protect a malicious same-UID host. The caller
must authenticate and protect executable, current inputs, policy and trusted
head; a head copied from the receipt supplies no independent trust.

## Validation status

The new public-interface test first failed because the SDK module did not exist.
The implementation and added integration tests now type-check on Rust 1.97.1.
Execution of the focused and native integration tests remains pending while the
original frozen gate uses the shared CLI binary. Do not count type-checking as
runtime validation. Katana's own native tests are separate evidence; they do not
establish this SDK integration.

An opt-in test, `actual_katana_turn_roundtrips_through_capture_and_current_resolution`,
requires `OW_KATANA_FIXTURE_PRODUCER` and `OW_KATANA_NATIVE_VERIFIER` absolute paths.
It uses an actual local Kernel turn with scripted responses and one bounded file
read. It checks import, current selection and requirement 12; excess capabilities,
changed log, wrong head and fake assurance must refuse. Missing current native
log must produce UNKNOWN. Sandbox and hard-spend requirements remain UNKNOWN.
Fixture head collection is test instrumentation, not protected production custody.
This copied example Warrant does not qualify its original business outcome.

Still open: protected collector enrollment, CLI/workflow configuration, native
child/fork coverage, reliable dollar accounting, sandbox evidence, participant
acceptance and independent qualification. Katana CI has unresolved baseline
formatting and dependency-policy failures. OW-WAR-0149 remains in progress.
