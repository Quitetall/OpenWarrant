# Usage data in Mission Control

t-3c16c W5 M2. Short by design: this note exists because the question touches the
journal format.

## What is recorded today

| source | where | what it holds |
|---|---|---|
| `war evidence go` | `go.ended` journal event | `tokens`: a number the harness reported, or `"unknown"`; `node`, `dispatch_id`, `attempt`. No model, no split between input and output |
| runtime capture | `<warrant>/runtime-receipts/capture-<digest>.json` | the provider's raw receipt: `input_tokens`, `output_tokens`, `usage_estimated`, `provider_model_identity` |

Nothing else records usage. `war start`, `war done` and an interactive session write no
tokens. Herdr reports a per-agent `tokens` object, but it is live, per-pane and gone
when the pane is, so it is no record.

## What the app shows

Only those two, read with no write and no change to any record:

- A total is shown when at least one run has a number. Runs that reported none are
  counted beside it: `TOKENS 4242 (2 run(s) UNKNOWN)`. With no number at all, `UNKNOWN`.
  Zero is never shown for "nothing recorded".
- A receipt flagged `usage_estimated` is the provider's estimate. It is kept out of the
  total and shown beside it as `est.`.
- A capture whose dispatch already ended with a `go.ended` count is the same run, and
  is not counted twice.
- A capture that fails `runtime_capture::inspect_retained` is not read. It is reported
  as a fault. A capture that reads is still unverified provider data: nothing here
  grants it assurance.

## Dollars

Only from a dated price table in `openwarrant.toml`, and always labelled an estimate:

```toml
[usage]
prices_as_of = "2026-11-01"

[usage.prices."claude-opus-5.5"]
input_per_mtok = 5.0
output_per_mtok = 25.0
```

A table with no `prices_as_of` is refused as a fault and no dollars show. A price applies
only to a receipt naming that model exactly, with both counts. `go.ended` holds neither a
model nor a split, so its tokens are never priced: they are listed as unpriced, not
averaged. There is no built-in price. Prices go stale, and a default would be a fabricated
number with a real-looking date.

## Not done here: recording session usage

Most work is an interactive session that records nothing. Recording it does touch the
journal format, and is left for a signing sitting because `work.*` events are normative
(FORMAT 11.2). The harness-neutral shape, when that is decided:

- A new optional event, `work.usage`, written by `war done` on the holder's item:
  `{target, tokens, input?, output?, model?, source}`. `source` names who reported it
  (`claude-hook`, `harness`, `self-reported`). Absent, usage stays UNKNOWN.
- Any harness can supply it with `war done --usage tokens=N[,model=M]`, or
  `OPENWARRANT_USAGE` set by a hook. Claude Code's `Stop` hook has the numbers;
  a harness without one reports nothing, and that is still a true state.
- It never gates anything. A missing or wrong number blocks no `war done`.

The app reads such an event the way it reads `go.ended`, with no other change.
