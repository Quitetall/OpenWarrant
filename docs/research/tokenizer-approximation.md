# Token estimates for Dispatch budgets

The question is which local approximation to retain for Dispatch budgets:
bytes / 4 or words × 13 / 10.

**Decision:** keep `oh.war/token-estimate/bytes-div-4/v1`: count UTF-8 bytes,
divide by four and round up. The method in
[tokens.rs](../../crates/openwarrant-core/src/tokens.rs) is unchanged.

This is an estimate, not a tokenizer measurement or an enforced usage limit.
The observations below compare two estimates over four named source files.
They show disagreement between formulas. They do not show which formula is
closer to a model's actual tokens, or prove that either estimate is an upper bound.

## Exact source observations

The historical source is the commit that first added this memo. The current
source is a separate, named revision. They are not one collection time.
Every count uses `LC_ALL=C wc -c` or `LC_ALL=C wc -w` over the exact Git blob.
A word is what that `wc` invocation counts, not a model token or a language unit.
Both comparison columns use integer ceiling; no floating-point rounding is used.

Set these variables once for the per-row replay commands:

```sh
token_count_measure=docs/warrants/OW-WAR-0065/implementation/tokenizer-counts-20261011/reproduce.py
token_count_historical=46d5d26635f53bc39a617f0664aa4608a490b97d
token_count_current=a6b2d90ca939d6f80a648b9d9a8db93893bbe816
```

### Historical source

Commit `46d5d26635f53bc39a617f0664aa4608a490b97d`.

| File | Bytes | Words | ceil(bytes / 4) | ceil(words × 13 / 10) | Replay all four figures |
| --- | ---: | ---: | ---: | ---: | --- |
| `docs/sas/WAR_Software_Architecture_Specification.md` | 143,180 | 19,665 | 35,795 | 25,565 | `python3 "$token_count_measure" --commit "$token_count_historical" --path docs/sas/WAR_Software_Architecture_Specification.md` |
| `docs/sas/generated/NORMATIVE.md` | 49,609 | 7,132 | 12,403 | 9,272 | `python3 "$token_count_measure" --commit "$token_count_historical" --path docs/sas/generated/NORMATIVE.md` |
| `crates/openwarrant-cli/src/check.rs` | 64,698 | 6,077 | 16,175 | 7,901 | `python3 "$token_count_measure" --commit "$token_count_historical" --path crates/openwarrant-cli/src/check.rs` |
| `docs/warrants/OW-WAR-0064/atoms/10-intent.md` | 2,811 | 413 | 703 | 537 | `python3 "$token_count_measure" --commit "$token_count_historical" --path docs/warrants/OW-WAR-0064/atoms/10-intent.md` |

### Current source

Commit `a6b2d90ca939d6f80a648b9d9a8db93893bbe816`.

| File | Bytes | Words | ceil(bytes / 4) | ceil(words × 13 / 10) | Replay all four figures |
| --- | ---: | ---: | ---: | ---: | --- |
| `docs/sas/WAR_Software_Architecture_Specification.md` | 147,586 | 20,379 | 36,897 | 26,493 | `python3 "$token_count_measure" --commit "$token_count_current" --path docs/sas/WAR_Software_Architecture_Specification.md` |
| `docs/sas/generated/NORMATIVE.md` | 51,800 | 7,513 | 12,950 | 9,767 | `python3 "$token_count_measure" --commit "$token_count_current" --path docs/sas/generated/NORMATIVE.md` |
| `crates/openwarrant-cli/src/check.rs` | 129,989 | 12,379 | 32,498 | 16,093 | `python3 "$token_count_measure" --commit "$token_count_current" --path crates/openwarrant-cli/src/check.rs` |
| `docs/warrants/OW-WAR-0064/atoms/10-intent.md` | 2,811 | 413 | 703 | 537 | `python3 "$token_count_measure" --commit "$token_count_current" --path docs/warrants/OW-WAR-0064/atoms/10-intent.md` |

## Reproduce or inspect offline

The [reproduction script](../warrants/OW-WAR-0065/implementation/tokenizer-counts-20261011/reproduce.py)
reads the four files from the selected commit, hashes their bytes and runs the
counts. It writes JSON to stdout and changes no repository file. A missing
revision or source fails; it never becomes a measured zero.

```sh
python3 docs/warrants/OW-WAR-0065/implementation/tokenizer-counts-20261011/reproduce.py --commit 46d5d26635f53bc39a617f0664aa4608a490b97d
python3 docs/warrants/OW-WAR-0065/implementation/tokenizer-counts-20261011/reproduce.py --commit a6b2d90ca939d6f80a648b9d9a8db93893bbe816
```

Retained counts, source hashes and byte-for-byte source archives are in
[the observation record](../warrants/OW-WAR-0065/implementation/tokenizer-counts-20261011/observation.json).
The archives include `manifest.json` and preserve the four original source paths.
A reviewer can rerun a count without the historical Git object, for example:

```sh
tar -xOzf docs/warrants/OW-WAR-0065/implementation/tokenizer-counts-20261011/historical-sources.tar.gz docs/sas/generated/NORMATIVE.md | LC_ALL=C wc -c
tar -xOzf docs/warrants/OW-WAR-0065/implementation/tokenizer-counts-20261011/historical-sources.tar.gz docs/sas/generated/NORMATIVE.md | LC_ALL=C wc -w
```

For each row, the formulas are `(bytes + 3) // 4` and `(13 * words + 9) // 10`.
The captured JSON names the same source commit and per-file SHA-256 values.
Archive digests are retained in `observation.json`. These performer observations
identify the measured bytes; they do not authenticate a reviewer or award assurance.

## Why retain the method

For all eight observed file versions, bytes / 4 gives the larger estimate.
Within this comparison, it leaves more estimated budget margin. It also uses a
byte count without a word-splitting rule, and existing packets already name it.
These are reasons to retain a reproducible, compatible approximation. They are
not evidence of actual tokenizer accuracy. No model or tokenizer ran.

The prior memo's byte estimates for 49,609, 64,698 and 2,811 bytes rounded down
instead of up. The corrected estimates are 12,403, 16,175 and 703. Its original
bytes and declared delivery record remain in `previous-memo.md.gz` and
`previous-deliverables.toml`; earlier review records are preserved. This correction
does not give those records a review of the new memo.

A real tokenizer or a different approximation needs its own method identity and
accuracy observations. Estimates from different methods must remain distinguished.
Independent review of these figures and the decision is still required by OW65.
