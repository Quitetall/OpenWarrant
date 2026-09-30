# Which tokenizer approximation OpenWarrant uses for §33.7 budgets

**Question.** A Dispatch records a token estimate and a budget (SAS §33.7);
the normative projection reports its size. No model tokenizer is part of the
tool, so the estimate is an approximation. Two are in common use: **bytes ÷ 4**
and **words × 1.3**. Which one, and why?

**Decision.** Bytes ÷ 4, rounded up — the method already named
`oh.war/token-estimate/bytes-div-4/v1` in
[`tokens.rs`](../../crates/openwarrant-core/src/tokens.rs).
It over-estimates prose by roughly a third and Rust source by more, which is
the right direction for a budget, and it needs no notion of "word" for YAML,
JSON or code.

## Measurements

Taken at the commit that authored this Warrant, on the files this repository
actually dispatches and projects:

| file | bytes | words | bytes÷4 | words×1.3 | bytes/word |
|---|---|---|---|---|---|
| `docs/sas/WAR_Software_Architecture_Specification.md` | 143,180 | 19,665 | 35,795 | 25,564 | 7.28 |
| `docs/sas/generated/NORMATIVE.md` | 49,609 | 7,132 | 12,402 | 9,271 | 6.96 |
| `crates/openwarrant-cli/src/check.rs` | 64,698 | 6,077 | 16,174 | 7,900 | 10.65 |
| `docs/warrants/OW-WAR-0064/atoms/10-intent.md` | 2,811 | 413 | 702 | 536 | 6.81 |

Commands: `wc -c <file>`, `wc -w <file>`; the derived columns are the two
formulas applied to those counts.

## Reading

- On English prose (the SAS, an intent atom) bytes÷4 exceeds words×1.3 by
  about 30–40%. Published tokenizer ratios for English prose sit near 1.3
  tokens per word and 4 characters per token, so both are in range and
  bytes÷4 is the conservative one.
- On Rust source the two diverge by 2×: whitespace and punctuation are
  bytes but not words. A budget that under-counts code by half is not a
  budget; bytes÷4 stays close to what a tokenizer produces on code.
- Structured atoms (`45-milestones.yaml`) and JSON packets have no honest
  word count at all.

## Consequence

The method id stays `bytes-div-4/v1`. A future method — a real tokenizer,
or a per-language table — gets a new id, and every record that carries an
estimate says which one produced it, so no two estimates are compared
across methods without saying so. The estimate is labelled an estimate
wherever it appears (`war dispatch`, `NORMATIVE.md`'s header, the
verification bundle).
