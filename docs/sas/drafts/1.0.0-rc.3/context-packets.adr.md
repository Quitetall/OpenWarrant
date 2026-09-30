# Historical context-packet decision pointer

The earlier context-compilation proposal is preserved in
[RC.2](../1.0.0-rc.2/context-packets.adr.md). RC.3 retains its exact-content,
explicit-input, conservative-selection and offline-profile semantics while moving
implementation to LAMU or another provider. Current ownership decision:
[SDK architecture amendment](sdk-ownership.adr.md). This reference neither signs
an ADR nor changes the historical source or its acceptance standing.

The current named views, shared-project contracts, query boundary and retention
semantics are consolidated in [context views and shared work](context-views-and-shared-work.md),
with change/resume handling in [work-stop contract](work-stop-contract.md).
