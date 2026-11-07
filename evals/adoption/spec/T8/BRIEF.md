# Monthly budget tables

## The request

`budget` prints one table for a whole period. Users budget by the month and want to
see each month on its own. Add a way to get one budget table per calendar month of a
period.

## The contract

These are the outside facts the work must meet. Anything not stated here is yours to
decide, or to ask the customer about.

- **C1.** The new form is `python3 -m tally budget <journal> --from DATE --until DATE --monthly`.
  It exits 0 when the journal is good. Without `--monthly`, `budget` behaves exactly as it does today.
- **C2.** The period is cut into calendar months, called blocks. The first block starts at
  `--from` and the last ends at `--until`, so the first and last blocks may be partial
  months, and every other block is a whole month. When both dates fall in the same month
  there is a single block.
- **C3.** Blocks print in date order. Each block is a line `FIRST..LAST`, the block's first and
  last dates, exactly `YYYY-MM-DD..YYYY-MM-DD` (two dots, no spaces), followed by a table in
  exactly today's `budget` format: its header line, then its rows.
- **C4.** Blocks are separated by one empty line, with none after the last. The output ends with
  a newline, like every report.
- **C5.** Every block lists the same rows: every account and commodity that any periodic
  transaction budgets, including in a month where its rule has no occurrence. Rows are in
  today's `budget` order: by account name and, within one account, the default commodity
  first and then by commodity code.
- **C6.** A block's Actual is today's Actual over the block's dates. Its Remaining is its Budget
  minus its Actual.
- **C7.** A rule occurrence belongs to the block its date falls in, so a yearly rule budgets only
  the month of its occurrence. Which postings budget, and how much each occurrence budgets,
  is what `budget` does today.
- **C8.** One decision isn't in this brief: how each month's Budget figure is worked out. The
  customer holds it. Ask.
- **C9.** `--monthly` keeps every `budget` error. With `--from` after `--until` it exits with status 2,
  prints nothing on stdout, and stderr is exactly `tally: error: --from must not be after --until`.
  With `--from` or `--until` missing it exits with status 2 and prints nothing on stdout.
