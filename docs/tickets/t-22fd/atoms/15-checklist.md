# Checklist

- [x] Show exactly which paths a signature writes (i-8339) — done by claude, 2026-09-25: Derived per act from sign.rs, authorize.rs, resolution_cmd.rs, correct.rs, sas.rs, roadmap_cmd.rs, batch_cmd.rs, invalidation.rs, standing_cmd.rs, questions.rs, attest.rs; listed in the ticket note
- [x] Decide: exclude authority records from the tree binding, or declare gate inputs (i-3499) — done by claude, 2026-09-25: Exclude authority records from the tree fallback only; declared inputs unchanged. Reasoning in the ticket note and docs/RESOLVING.md
- [x] Implement with an accepting and a refusing plant (i-71d8) — done by claude, 2026-09-25: source::is_authority_record + Exclusions::excludes_from_tree (tree rule only); unit test; plant 55-evidence-signing.sh accept + refuse, verified failing on the old binary; subset of 10 plant files 149 passed 0 failed
