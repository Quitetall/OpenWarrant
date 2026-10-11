# Katana public CLI transport — implementation observation

Shared work: OW-WAR-0149 (`01a0f502-4941-70a1-a446-e1eb77dff191`).
Katana source `6d89a10b79d841c8a2b2bbd3d1e8e1ee413f9894`, based on merged `9f2f531e1af29cf07e510c11dd9c6dd9de02d6ef`; [draft proposal](https://github.com/Quitetall/katana/pull/14). This is a proposed participant change, not participant acceptance.

The public one-shot CLI now uses the existing bound native kernel interface. It emits the existing receipt format and preserves failed terminal state. Invalid/duplicate/oversized bindings, resumed sessions, existing output, and log/output collision refuse before inference. Native wire format, digest domains, permissions and cost provenance remain unchanged. Caller admission and exact compiled context delivery remain the workflow's responsibility.

Final checks: six CLI tests plus eight native receipt tests PASS; one child instrumentation entry ignored. Workspace fmt and all-target/all-feature Clippy with warnings denied PASS on Rust 1.99.0; Rust 1.88.0 all-target/all-feature workspace check PASS. Local synthetic provider responses exercise real public binary and native verifier. No real model call, measured dollar cost, confinement qualification, human signature or independent assurance is claimed.

Three retained red logs show the missing flags, inference before existing-output refusal, and inference before path-collision refusal; corresponding green controls and final logs are alongside this note. The failed-turn log verifies retained failed native evidence. Output publication is exclusive creation, not a transaction or same-identity isolation boundary. Protected collector enrollment, participant agreement, native child lineage and real execution qualification remain open.
