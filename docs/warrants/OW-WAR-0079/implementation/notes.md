# OW-WAR-0079 implementation

In progress, unverified. LAMU provider commit
4c88530f2b84072ea191132e2ed0bb91930b3bf9 implements deterministic selection and
bounded dependency closure. OpenWarrant owns the shared contract and driver;
no second closure engine was added to the SDK.

Six public selection tests cover T21-T26 plus access, required UNKNOWN inclusion,
raw versus parsed whole-source references, opaque bytes, master ordering and
resource refusal. All sixteen provider tests and Clippy pass. The attached
observation binds exact source, fixture, Cargo.lock and profile identities.

Independent Spec and Standards reviews passed after reproducible repairs:
whole-file bytes were lost during expansion, absent optional targets appeared
included, master mode selected raw blobs, and preflight/traversal allocations
were not fully charged. Regression tests and scratch probes confirmed repairs.
A 2003-unit quota probe now refuses after 295 cumulative allocated bytes rather
than 165492. This is cumulative allocation, not peak-memory measurement.

LAMU commit review returned PASS WITH NITS. Its hypothetical #* count overflow
was checked: source unit IDs are unique and the preflight missing count equals
new entries; per-entry quota checks remain. Cycle ancestor scans are linear but
charged to the explicit visit quota. Defaults are specified in the shared contract.
No additional fix was justified by those findings.

This is an in-process selection primitive, not full F5 compilation, packet export,
a trusted access service or human assurance. Participant merges and broader
integration remain pending. 0080 consumes this exact interface; 0081 owns packet
integrity and offline export; 0082 owns full budget/cache behavior.
