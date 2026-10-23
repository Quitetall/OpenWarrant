"""Hidden acceptance tests for T8: budget --monthly, carrying each month's remainder forward."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t8.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


def row(account, actual, budget, remaining):
    return f"{account:<30}{actual:>14}{budget:>14}{remaining:>14}"


HEADER = row("Account", "Actual", "Budget", "Remaining")

JOURNAL = """
~ monthly from 2026-01-01 Groceries
    Expenses:Food  400
    Assets:Checking

2026-01-10 Market
    Expenses:Food:Groceries  150
    Assets:Checking

2026-02-10 Market
    Expenses:Food  500
    Assets:Checking

2026-03-03 Market
    Expenses:Food  100
    Assets:Checking
"""


def test_three_months_carry_the_remainder_forward(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-01-01", "--until", "2026-03-31", "--monthly")
    assert result.returncode == 0, result.stderr
    # Jan: 400 - 150 = 250 left. Feb: 400 + 250 = 650, spent 500, 150 left.
    # Mar: 400 + 150 = 550, spent 100.
    assert result.stdout == "\n".join([
        "2026-01-01..2026-01-31",
        HEADER,
        row("Expenses:Food", "150.00", "400.00", "250.00"),
        "",
        "2026-02-01..2026-02-28",
        HEADER,
        row("Expenses:Food", "500.00", "650.00", "150.00"),
        "",
        "2026-03-01..2026-03-31",
        HEADER,
        row("Expenses:Food", "100.00", "550.00", "450.00"),
    ]) + "\n"


def test_overspending_reduces_the_next_month(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-01 Fun
            Expenses:Fun  100
            Assets:Checking

        2026-01-20 Concert
            Expenses:Fun  180
            Assets:Checking
        """)
    result = tally("budget", path, "--from", "2026-01-01", "--until", "2026-02-28", "--monthly")
    assert result.returncode == 0, result.stderr
    lines = result.stdout.splitlines()
    assert lines[2] == row("Expenses:Fun", "180.00", "100.00", "-80.00")
    assert lines[6] == row("Expenses:Fun", "0.00", "20.00", "20.00")


def test_partial_first_and_last_months(tmp_path):
    path = write(tmp_path, """
        ~ weekly from 2026-01-01 Lunch
            Expenses:Lunch  10
            Assets:Checking

        2026-01-16 Lunch
            Expenses:Lunch  12
            Assets:Checking

        2026-03-12 Lunch
            Expenses:Lunch  99
            Assets:Checking
        """)
    result = tally("budget", path, "--from", "2026-01-15", "--until", "2026-03-10", "--monthly")
    assert result.returncode == 0, result.stderr
    # Weekly from Jan 1: Jan 15, 22, 29 | Feb 5, 12, 19, 26 | Mar 5.
    assert result.stdout.splitlines() == [
        "2026-01-15..2026-01-31",
        HEADER,
        row("Expenses:Lunch", "12.00", "30.00", "18.00"),
        "",
        "2026-02-01..2026-02-28",
        HEADER,
        row("Expenses:Lunch", "0.00", "58.00", "58.00"),
        "",
        "2026-03-01..2026-03-10",
        HEADER,
        row("Expenses:Lunch", "0.00", "68.00", "68.00"),
    ]


def test_one_month_gives_one_block(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-02-05", "--until", "2026-02-20", "--monthly")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "2026-02-05..2026-02-20",
        HEADER,
        row("Expenses:Food", "500.00", "0.00", "-500.00"),
    ]


def test_every_block_lists_every_budgeted_account(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-01 Groceries
            Expenses:Food  400
            Assets:Checking

        ~ yearly from 2026-02-15 Insurance
            Expenses:Insurance  300
            Assets:Checking

        2026-03-01 Insurer
            Expenses:Insurance  250
            Assets:Checking
        """)
    result = tally("budget", path, "--from", "2026-01-01", "--until", "2026-03-31", "--monthly")
    assert result.returncode == 0, result.stderr
    lines = result.stdout.splitlines()
    assert lines[1:4] == [
        HEADER,
        row("Expenses:Food", "0.00", "400.00", "400.00"),
        row("Expenses:Insurance", "0.00", "0.00", "0.00"),
    ]
    assert lines[7:9] == [
        row("Expenses:Food", "0.00", "800.00", "800.00"),
        row("Expenses:Insurance", "0.00", "300.00", "300.00"),
    ]
    assert lines[12:14] == [
        row("Expenses:Food", "0.00", "1200.00", "1200.00"),
        row("Expenses:Insurance", "250.00", "300.00", "50.00"),
    ]


def test_carry_is_per_commodity(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-01 Travel
            Expenses:Travel  200 EUR
            Expenses:Travel  50
            Assets:Checking  -200 EUR
            Assets:Checking  -50

        2026-01-12 Train
            Expenses:Travel  120 EUR
            Assets:Checking
        """)
    result = tally("budget", path, "--from", "2026-01-01", "--until", "2026-02-28", "--monthly")
    assert result.returncode == 0, result.stderr
    # Both sides state their amounts, so both are budgeted, per commodity;
    # within an account the default commodity comes first.
    assert result.stdout.splitlines() == [
        "2026-01-01..2026-01-31",
        HEADER,
        row("Assets:Checking", "0.00", "-50.00", "-50.00"),
        row("Assets:Checking", "-120.00 EUR", "-200.00 EUR", "-80.00 EUR"),
        row("Expenses:Travel", "0.00", "50.00", "50.00"),
        row("Expenses:Travel", "120.00 EUR", "200.00 EUR", "80.00 EUR"),
        "",
        "2026-02-01..2026-02-28",
        HEADER,
        row("Assets:Checking", "0.00", "-100.00", "-100.00"),
        row("Assets:Checking", "0.00 EUR", "-280.00 EUR", "-280.00 EUR"),
        row("Expenses:Travel", "0.00", "100.00", "100.00"),
        row("Expenses:Travel", "0.00 EUR", "280.00 EUR", "280.00 EUR"),
    ]


def test_without_monthly_budget_is_unchanged(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-01-01", "--until", "2026-03-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [HEADER, row("Expenses:Food", "750.00", "1200.00", "450.00")]


def test_from_after_until_is_refused(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-03-01", "--until", "2026-01-31", "--monthly")
    assert result.returncode == 2
    assert result.stdout == ""
    assert result.stderr == "tally: error: --from must not be after --until\n"


def test_monthly_still_requires_both_dates(tmp_path):
    result = tally("budget", write(tmp_path, JOURNAL), "--from", "2026-01-01", "--monthly")
    assert result.returncode == 2
    assert result.stdout == ""
