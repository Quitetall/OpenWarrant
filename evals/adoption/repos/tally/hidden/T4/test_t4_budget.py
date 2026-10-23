"""Hidden acceptance tests for T4: the `budget` report."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t4.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


def row(account, actual, budget, remaining):
    return f"{account:<30}{actual:>14}{budget:>14}{remaining:>14}"


HEADER = row("Account", "Actual", "Budget", "Remaining")

JOURNAL = """
~ monthly from 2026-01-01 Household
    Expenses:Food  400
    Expenses:Rent  1200
    Assets:Checking

~ weekly from 2026-01-01 Allowance
    Expenses:Kids  10
    Assets:Cash

2026-01-05 Market
    Expenses:Food:Groceries  120.50
    Assets:Checking

2026-01-06 Landlord
    Expenses:Rent  1200
    Assets:Checking

2026-01-20 Restaurant
    Expenses:Food:Dining  95.25
    Assets:Checking

2026-02-03 Market
    Expenses:Food  300
    Assets:Checking

2026-02-04 Landlord
    Expenses:Rent  1200
    Assets:Checking

2026-03-01 Market
    Expenses:Food  999
    Assets:Checking

2026-01-15 Salary
    Assets:Checking  3000
    Income:Salary
"""


def test_budget_report_exact_output(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-01-01", "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    # Weekly from 2026-01-01: Jan 1, 8, 15, 22, 29 -> 5 occurrences of 10.
    assert result.stdout == "\n".join([
        HEADER,
        row("Expenses:Food", "215.75", "400.00", "184.25"),
        row("Expenses:Kids", "0.00", "50.00", "50.00"),
        row("Expenses:Rent", "1200.00", "1200.00", "0.00"),
    ]) + "\n"


def test_budget_sums_occurrences_and_actuals_inside_the_period_only(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-01-10", "--until", "2026-02-28")
    assert result.returncode == 0, result.stderr
    # Monthly occurrences in range: Feb 1 only. Weekly: Jan 15, 22, 29, Feb 5, 12, 19, 26 -> 7.
    assert result.stdout.splitlines() == [
        HEADER,
        row("Expenses:Food", "395.25", "400.00", "4.75"),
        row("Expenses:Kids", "0.00", "70.00", "70.00"),
        row("Expenses:Rent", "1200.00", "1200.00", "0.00"),
    ]


def test_over_budget_gives_negative_remaining(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-03-01", "--until", "2026-03-01")
    assert result.returncode == 0, result.stderr
    assert row("Expenses:Food", "999.00", "400.00", "-599.00") in result.stdout.splitlines()


def test_funding_posting_and_unbudgeted_accounts_are_not_rows(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-01-01", "--until", "2026-12-31")
    assert result.returncode == 0, result.stderr
    accounts = [line.split()[0] for line in result.stdout.splitlines()[1:]]
    assert accounts == ["Expenses:Food", "Expenses:Kids", "Expenses:Rent"]


def test_income_budget_with_explicit_amounts_on_both_sides(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-25 Payday
            Assets:Checking  3000
            Income:Salary  -3000

        2026-01-25 Employer
            Assets:Checking  3100
            Income:Salary
        """)
    result = tally("budget", path, "--from", "2026-01-01", "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        HEADER,
        row("Assets:Checking", "3100.00", "3000.00", "-100.00"),
        row("Income:Salary", "-3100.00", "-3000.00", "100.00"),
    ]


def test_no_periodic_transactions_prints_only_the_header(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Market
            Expenses:Food  10
            Assets:Checking
        """)
    result = tally("budget", path, "--from", "2026-01-01", "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout == HEADER + "\n"


def test_from_after_until_refused(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-02-01", "--until", "2026-01-01")
    assert result.returncode == 2
    assert result.stderr == "tally: error: --from must not be after --until\n"
    assert result.stdout == ""


def test_both_dates_are_required(tmp_path):
    path = write(tmp_path, JOURNAL)
    assert tally("budget", path, "--from", "2026-01-01").returncode == 2
    assert tally("budget", path, "--until", "2026-01-31").returncode == 2


def test_invalid_date_refused(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-01-01", "--until", "2026-01-32")
    assert result.returncode == 2
    assert "invalid date '2026-01-32'" in result.stderr
