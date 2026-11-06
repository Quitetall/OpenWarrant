# Journal format

A journal is a UTF-8 text file read line by line.

## Comments and blank lines

A line whose first non-blank character is `;` or `#` is a comment and may
appear anywhere, including between the postings of a transaction. Text after
`;` or `#` on a posting line is an inline comment. A blank line ends the
current transaction.

## Transactions

A transaction starts with a header in column one: a date, a space, and the
payee (the rest of the line, which may be empty).

```text
2026-01-05 Corner Grocery
    Expenses:Food:Groceries   42.50
    Assets:Checking
```

Dates are `YYYY-MM-DD` and must be real calendar dates.

Each following indented line (at least one space or tab) is a posting: an
account name, then **two or more spaces or a tab**, then an amount. Because the
separator is wider than one space, account names may contain single spaces
(`Assets:Joint Checking`).

A transaction needs at least two postings, and its amounts must sum to
exactly zero. One posting may leave its amount out; it receives whatever
balances the transaction.

## Accounts

Account names are segments joined by `:`, forming a hierarchy:
`Expenses:Food:Groceries` is a child of `Expenses:Food`. No segment may be
empty or begin or end with whitespace.

## Amounts

An amount is a decimal number with an optional sign: `42`, `-3.50`, `.25`.
Amounts keep the precision they were written with and are added exactly.
Reports display them rounded to two places, half to even.

## Problems

`tally check` reports every problem it finds as `FILE:LINE: message`, where
`FILE` is the path as given on the command line. A transaction with an
unparseable line is reported once, at that line, and is not also checked for
balance.
