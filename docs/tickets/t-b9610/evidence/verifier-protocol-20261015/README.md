# Verifier protocol prompt correction

At implementation commit `2ca41075`, `bundle.rs::SCHEMA` and the adapter's
system instructions name `oh.war/verification-bundle/v2`. The final `-p`
instruction previously named v1. Only that instruction changes to v2.

`bash -n tools/verifier/claude-verifier.sh` passes. The existing
`conformance/plants.d/62-verifier.sh` suite reports **17 passed, 0 failed** in
`wrapper-controls.log`, run through `conformance/lib.sh` with the unchanged
previously built Rust 1.97.1 CLI from the Katana consumer checkout. This is not
a build of the alpha.4 CLI. The suite substitutes a fake Claude executable;
no model inference, paid call or production verdict occurred. These controls
exercise adapter refusals and output handling, not a model's interpretation
of the prompt. The protocol mismatch itself is established by source inspection.

The ticket's broader context closure, custody and independent qualification
items remain open. No signature or assurance status changes.
