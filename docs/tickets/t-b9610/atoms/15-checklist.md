# Checklist

- [ ] Bind verifier requests and returned results to exact reviewed contract and artifact snapshot (i-37a7)
- [ ] Reject old responses and exclude stale or unbound verdicts from current preparation and resolution (i-c952)
- [x] Prove public CLI refusal and actual configured-verifier refresh without rewriting history (i-4889) — done by codex, 2026-10-09: Public CLI stale-response refusal and actual configured-verifier refresh proved on a synthetic scratch corpus with two real local Qwen reviews through LAMU 0.6.0; previous record bytes retained, authority unchanged, missing execution not_established. Evidence: evidence/local-verifier-live/README.md. Seven synthetic transport controls pass. New optional local client refuses in-repository logs, redirects, model substitution, tool calls and truncated answers; no default policy changed or paid call made. This is bounded transport/history proof, not production qualification; broader ticket items remain open.
