# Source provider integration cases (Phase 2, pending)

OW-WAR-0077 owns the OpenWarrant side. A real source provider owns acquisition and
reference resolution. This directory defines the required observations; it does
not claim a transport, provider, passing fixture or supported LAMU profile.

| Case | Real-provider observation required |
| --- | --- |
| T11 | Capture a Warrant, ADR and opaque fixture; each pin matches original bytes, document identity and unit ranges. SDK checks the returned descriptors and references against supplied blobs. |
| T12 | Rename a heading while preserving unit ID; resolve against the new captured basis. Old reference and approval subjects remain unchanged. |
| T13 | Missing target, ambiguous identity and wrong digest produce distinct refusal diagnostics; no complete output is published. |
| T14 | Reject traversal, absolute paths and prohibited symlinks at the real I/O boundary. Change a source during capture and observe snapshot refusal with prior output preserved. |
| T15 | Reorder capture enumeration and obtain identical canonical source locks and deterministic provider outputs. |

Before claiming integration, fix provider identity/build, versioned capability
and request/response profile, access basis and limits in a shared cross-project
contract. Bind actual provider and SDK revisions; retain positive/refusal outputs.
No field claiming permission or success is trusted without the caller's trusted
adapter. A fake provider can test response handling only and cannot qualify LAMU.
The SDK never takes over dependency closure, ranking, assembly or package creation.
