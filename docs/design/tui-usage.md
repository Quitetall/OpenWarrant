# Usage data in Mission Control

t-3c16c W5 M2. Short by design: this note exists because the question touches the
journal format.

## What is recorded

| source | where | what it holds |
|---|---|---|
| `war evidence go` | `go.ended` journal event | `tokens`: a number the harness reported, or `"unknown"`; `node`, `dispatch_id`, `attempt`. No model, no split between input and output |
| runtime capture | `<warrant>/runtime-receipts/capture-<digest>.json` | the provider's raw receipt: `input_tokens`, `output_tokens`, `usage_estimated`, `provider_model_identity` |
| a work session | `work.usage` journal event, written by `war done` (and by `war start` handing a session over) | per model: `input`, `output`, `cache_read`, `cache_write`, `tokens`; `source`, `session` (a digest), `at`, `from`. Or `tokens: "unknown"` with a `reason` |

Herdr reports a per-agent `tokens` object, but it is live, per-pane and gone when the
pane is, so it is no record. It is not shown.

## `work.usage`: a session's usage

When `war done` ticks work (not a refusal, not an item already done), it reads the
harness's own local record of the session it runs in and journals one event on the
ticket:

```json
{"target": "t-61b4/i-cbfa", "source": "claude-code", "session": "sha256:…",
 "at": "2026-11-07T12:38:53Z", "from": "2026-11-07T12:38:52Z", "tokens": 145,
 "models": [{"model": "claude-x", "input": 10, "output": 5, "cache_read": 100,
             "cache_write": 20, "tokens": 135}, …]}
```

`tokens` is input, output and cache tokens together; the split is beside it. With no
record to read it journals `{"target", "source", "session", "at", "from", "tokens":
"unknown", "reason"}`, and the reason names no path and no session id. It is never 0:
a window in which the record holds no usage is `"unknown"` too.

Which harness, from the session variable it sets:

| harness | variable | what is read |
|---|---|---|
| Claude Code | `CLAUDE_CODE_SESSION_ID` | `<CLAUDE_CONFIG_DIR or ~/.claude>/projects/*/<session>.jsonl` and `<session>/subagents/*.jsonl`: each `assistant` line's `message.usage` and `message.model`. One API message is written as a line per content block, each with the same usage, so a message id counts once |
| Codex | `CODEX_THREAD_ID` | `<CODEX_HOME or ~/.codex>/sessions/**/rollout-*-<thread>.jsonl`: the running `total_token_usage` of `token_count` events, less the total at the window's start; the model from `turn_context`. Two models in one window name none, so that window is not priced |
| opencode, others | `CODEX_SESSION_ID`, `OPENWARRANT_SESSION_ID`, `KATANA_SESSION_ID` | nothing: `"unknown"`, naming the variable. opencode keeps its sessions in a SQLite database (`~/.local/share/opencode/opencode.db`) whose usage columns are not documented; it is not read. Such a harness reports with `--usage` |

With no session variable, nothing is journalled: there is no session to describe, and
absence reads as UNKNOWN. A `war evidence go` node (`WAR_GO_PACKET` set) is not read
either: its `go.ended` already counts that run. A replay (`war done --at`) reads
nothing.

**What a window is.** A per-session cursor in the claim store's directory
(`<git common dir>/openwarrant/usage-cursors/`, never committed) holds each file's byte
offset. `war start` marks the end of the session's files (a stat, no read) for the
ticket it takes. `war done` reads only what was appended since the mark, journals it,
and moves the mark; a line still being written waits for the next read. With no mark
(work claimed by `war admin claim`, or by a `war done` that claims), the files are read
once from the claim's `since` in the journal. A `war start` that takes other work while
the session's window is open on a ticket journals that window on the ticket first: the
hand-over. Everything one session spends between the mark and `war done` is in the
window, its subagents' included; two agents sharing one session id share one window.

**What is never read or kept.** Only `usage`, `model`, `id`, `type` and `timestamp` are
parsed from a line; a message's content is skipped by the parser and never held, and
no part of it enters the journal or the cursor. The session id is kept only as its
SHA-256. Nothing is sent over the network.

**Reporting it.** `war done --usage <file|->` journals a small JSON object instead of
reading any record: `{"model"?, "input"?, "output"?, "cache_read"?, "cache_write"?,
"tokens"?, "reporter"?}`, with `source: "reported"`. Another key, no count, or a
`tokens` that is not its parts is refused (`usage.report-invalid`) before anything is
ticked.

**Turning it off.** `[usage] record = false` in `openwarrant.toml`: `war done` and `war
start` read no record and journal nothing, and no cursor is written. An explicit
`--usage` is still journalled, because someone asked for it.

**What it is not.** The provider's numbers, unverified. They grant no assurance,
satisfy no obligation and gate nothing: a missing or wrong number refuses no `war
done`, and a failure to journal it is said beside the tick and undoes nothing.
`work.usage` is not in FORMAT-1.0 §11.2's table of core events; a reader that does
not know it carries it. Adding it there is a format decision, not made here.

## What the app shows

Only those three, read with no write and no change to any record:

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
- Watch: each live claim's recorded tokens (`tok UNKNOWN` until its work records some),
  then the most recently recorded work no live claim holds. Records, Usage: the total,
  then by ticket, actor, model and day. `war status --usage` prints the same in a few
  lines, and `--json` gives `oh.war/usage/v1`.

## Dollars

Only from a dated price table in `openwarrant.toml`, and always labelled an estimate:

```toml
[usage]
prices_as_of = "2026-11-01"

[usage.prices."claude-opus-5.5"]
input_per_mtok = 5.0
output_per_mtok = 25.0
cache_read_per_mtok = 0.5     # optional; without it, a run with cache reads is not priced
cache_write_per_mtok = 6.25   # optional; likewise for cache writes
```

A table with no `prices_as_of` is refused as a fault and no dollars show. A price applies
only to a run naming that model exactly, with both counts, and with a rate for each
cache count it has. `go.ended` holds neither a model nor a split, so its tokens are
never priced: they are listed as unpriced, not averaged. There is no built-in price.
Prices go stale, and a default would be a fabricated number with a real-looking date.
Every figure reads `~$1.23 est., prices of 2026-11-01`; a table more than 90 days old
reads `prices of 2026-01-01 (STALE, 310 days old)`.
