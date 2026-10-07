# Checklist

- [x] fixed kernel states, computed or authenticated, in the model (i-4639) — done by claude, 2026-10-07: core kernel_state (10 fixed states: kind, capability, RQ-032 facet); states.rs derives those that hold, capability-gated; war model emits them as kind computed|authenticated with facet, beside the builders' states; pack stays 0.2.0
- [x] declared states in profiles and war state (i-f998) — done by claude, 2026-10-07: role.rs [[states]] (refines-unknown/collides/unreachable/invalid refused); war state enters one as state.entered in the owning Warrant/ticket journal; holds while parent holds (item in_progress per claim), lapses after; authenticated parent must already hold; never read by any §56.1 check or gate; shown in war show/tickets
- [x] 75-states.sh (i-a886) — done by claude, 2026-10-07: conformance/plants.d/75-states.sh: 15 checks; battery in a clone 1322 passed, 0 failed
