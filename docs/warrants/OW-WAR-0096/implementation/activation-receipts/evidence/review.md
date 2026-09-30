# Activation receipt review

Source f5acf9c: independent spec PASS; standards PASS after additional refusal
plants and threat-model update. LAMU review_commit PASS WITH NITS using free local
Qwen fallback. Zero-sequence concern self-corrected: checked_sub refuses zero.
Signer ordering is deliberately exact and produced from sorted BTreeMap keys;
set-based normalization would hide altered stored observations. Wall clock is
explicitly unauthenticated. No source change required for those observations.

Rust 1.97.1: full gate 14/14 PASS; 308 plants PASS, 0 failures. Gate retains its
existing opt-in slow neighbor-timeout omission. Signature CLI control covers
subject/head/signer/sequence tampering, legacy absence and activating process UID.
This is reference software evidence, not production account/key custody proof.
