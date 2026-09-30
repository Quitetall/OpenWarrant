# First constrained observation

Exact adapter/source/model hashes are in identity.json. The local CPU-only model
returned complete structural output in 101.18 seconds. No paid service was used.
Raw backend traffic and proposal stdout are retained without repairs.

`war plan --proposal proposal.stdout.txt --json` passed its four non-applying
steps and reported application not ready. Inspection found invented OW99-specific
scope and malformed milestone YAML (`oh.war/milestones/v1` used as a mapping key,
not the schema field). This is not a usable reviewed draft. The four validation
steps do not establish complete embedded milestone validity or task fidelity.

No --reviewed or --apply was used. The task-owned server was terminated and its
process session exited zero. This text-only test does not establish containment
of the model service or independently reviewed drafting quality.

Follow-up changes constrain milestone YAML to a fixed adapter template and state
that inventory is background, not task scope. Those changes need a new observation.
