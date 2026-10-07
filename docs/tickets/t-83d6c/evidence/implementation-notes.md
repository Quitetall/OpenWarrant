# Dynamic scratch variables in runner controls

Three runner-control generators expanded their allocated mktemp paths into literal paths in generated shell sources. The existing fixed-path guard correctly refused those literals before the intended controls ran. Generated sources now reference the enclosing scratch variables at execution time. The guard and its refusal patterns are unchanged.

The actual runner and current controls ran in a separate no-hardlinks clone. Its generated corpus was compiled before the clean-tree guard. All ten observations passed, including the initial corpus check, fixed-path refusal at the intended line, quiet-grep refusal cases, duplicate prefixes, leak cleanup and reversed-order totals. The copied CLI was the pre-board-cache Rust 1.97.1 binary; this scope exercises shell controls, not a new Rust implementation. Full final-source integration gate remains required.

Completed unverified ticket work, not independent Warrant verification or human acceptance.
