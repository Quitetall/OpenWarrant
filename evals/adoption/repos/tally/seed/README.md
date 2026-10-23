# tally

A small plain-text double-entry ledger. You keep your transactions in a text
file; tally reads it, checks that every transaction balances, and prints
reports. Python 3.10+, standard library only.

## Usage

```sh
python3 -m tally <command> <journal file> [options]
```

| command | what it prints |
|---|---|
| `balance [ACCOUNT] [--from DATE] [--until DATE]` | net amount per account, sorted by name, then a total |
| `register [ACCOUNT] [--from DATE] [--until DATE]` | each posting in date order with a running total |
| `accounts` | every account name the journal uses |
| `print [--from DATE] [--until DATE]` | the transactions in date order, elided amounts filled in |
| `check` | `ok: ...`, or every problem in the journal as `FILE:LINE: message` |

`ACCOUNT` limits a report to that account and its descendants
(`Expenses:Food` includes `Expenses:Food:Groceries`). Dates are `YYYY-MM-DD`
and both ends of a range are inclusive.

Exit status is 0 on success, 1 when `check` finds problems, and 2 for a usage
error, an unreadable file, or a report asked of a journal that has problems.
Errors go to stderr as `tally: error: <message>`.

## Example

```text
$ cat household.ledger
2026-01-01 Salary
    Assets:Checking  2500.00
    Income:Salary

2026-01-05 Corner Grocery
    Expenses:Food:Groceries  42.50
    Assets:Checking

$ python3 -m tally balance household.ledger
       2457.50  Assets:Checking
         42.50  Expenses:Food:Groceries
      -2500.00  Income:Salary
--------------
          0.00
```

The journal syntax is described in [docs/FORMAT.md](docs/FORMAT.md).

## Layout

```text
tally/
  cli.py          argument parsing, one cmd_* function per command, exit codes
  parser.py       journal text -> Journal, collecting Problems with file:line
  validate.py     balancing and elided-amount inference for one transaction
  model.py        Posting, Transaction, Journal
  amounts.py      Decimal parsing, rounding and two-place formatting
  accounts.py     account-name rules and matching
  dates.py        YYYY-MM-DD parsing
  filters.py      date ranges and the --from/--until options
  writer.py       transactions back out as journal text
  reports/        balance, register and accounts: compute_* rows, render_* text
tests/            pytest suite
```

## Development

```sh
python3 -m pytest -q
```
