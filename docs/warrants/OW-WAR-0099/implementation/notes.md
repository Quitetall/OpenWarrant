# Frontier integrity result

Public CLI controls and Rust 1.97.1 full gate passed: 14/14 steps, 308 plants at 325ce50e242d2e13fc003b5e0dc40d5339a79af3. Initial gate failure came from stale unresolved OW68 delivery provenance; failed transcript retained compressed. New delivery records preserve the prior digest as an input. No signed contract or resolved pin changed.

This closes OW99 implementation only. Frontier remains a stage projection, not authority or complete admission. OW68 skill/runtime obligations remain open, and no independent assurance disposition is recorded.

## Scheduling payload follow-up

A valid journal envelope with malformed dispatch/submission payload was silently
ignored, potentially projecting claimed work as open. The new public CLI control
reproduced that defect. Frontier now refuses invalid JSON and missing, blank or
non-string stage fields for these two scheduling event types. Valid claim and
submission transitions and opaque unrelated events are tested. Historical full
gate evidence above applies only to its stated revision; this follow-up awaits
a fresh full gate.
