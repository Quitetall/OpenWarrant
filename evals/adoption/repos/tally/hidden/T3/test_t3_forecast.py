"""Hidden acceptance tests for T3: periodic transactions and `forecast --until`."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t3.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


def blocks(stdout):
    """Generated transactions as lists of lines, split on blank lines."""
    return [b.splitlines() for b in stdout.strip("\n").split("\n\n")] if stdout.strip() else []


def test_monthly_rule_expanded_with_elided_amount_filled(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-01 Rent
            Expenses:Rent  1200
            Assets:Checking
        """)
    result = tally("forecast", path, "--until", "2026-03-01")
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-01-01 Rent\n"
        "    Expenses:Rent  1200.00\n"
        "    Assets:Checking  -1200.00\n"
        "\n"
        "2026-02-01 Rent\n"
        "    Expenses:Rent  1200.00\n"
        "    Assets:Checking  -1200.00\n"
        "\n"
        "2026-03-01 Rent\n"
        "    Expenses:Rent  1200.00\n"
        "    Assets:Checking  -1200.00\n"
    )


def test_monthly_from_the_31st_clamps_then_returns(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-31 Card
            Liabilities:Card  50
            Assets:Checking
        """)
    result = tally("forecast", path, "--until", "2026-05-30")
    assert result.returncode == 0, result.stderr
    assert [b[0] for b in blocks(result.stdout)] == [
        "2026-01-31 Card", "2026-02-28 Card", "2026-03-31 Card", "2026-04-30 Card",
    ]


def test_weekly_and_yearly_periods(tmp_path):
    path = write(tmp_path, """
        ~ weekly from 2026-01-01 Allowance
            Expenses:Kids  5
            Assets:Cash

        ~ yearly from 2024-02-29 Insurance
            Expenses:Insurance  300
            Assets:Checking
        """)
    result = tally("forecast", path, "--until", "2026-01-15")
    assert result.returncode == 0, result.stderr
    assert [b[0] for b in blocks(result.stdout)] == [
        "2024-02-29 Insurance",
        "2025-02-28 Insurance",
        "2026-01-01 Allowance",
        "2026-01-08 Allowance",
        "2026-01-15 Allowance",
    ]


def test_same_date_occurrences_keep_file_order_and_default_description(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-05 Zeta
            Expenses:Z  1.5
            Assets:Cash

        ~ monthly from 2026-01-05
            Expenses:A  2
            Assets:Cash  -2
        """)
    result = tally("forecast", path, "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    assert blocks(result.stdout) == [
        ["2026-01-05 Zeta", "    Expenses:Z  1.50", "    Assets:Cash  -1.50"],
        ["2026-01-05 Forecast", "    Expenses:A  2.00", "    Assets:Cash  -2.00"],
    ]


def test_until_before_the_start_prints_nothing(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-06-01 Rent
            Expenses:Rent  1200
            Assets:Checking
        """)
    result = tally("forecast", path, "--until", "2026-05-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout == ""


def test_periodic_rules_do_not_affect_balance_or_check_counts(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-01 Rent
            Expenses:Rent  1200
            Assets:Checking

        2026-01-03 Salary
            Assets:Checking  100
            Income:Salary
        """)
    balance = tally("balance", path)
    assert balance.returncode == 0, balance.stderr
    assert "Expenses:Rent" not in balance.stdout
    check = tally("check", path)
    assert check.returncode == 0
    assert check.stdout == "ok: 1 transactions, 2 postings\n"


def test_forecast_output_is_a_valid_journal(tmp_path):
    path = write(tmp_path, """
        ~ weekly from 2026-01-01 Groceries
            Expenses:Food  80
            Assets:Wallet
        """)
    result = tally("forecast", path, "--until", "2026-02-01")
    out = tmp_path / "out.ledger"
    out.write_text(result.stdout, encoding="utf-8")
    check = tally("check", out)
    assert check.returncode == 0, check.stdout
    assert check.stdout == "ok: 5 transactions, 10 postings\n"


def test_unknown_period_reported(tmp_path):
    path = write(tmp_path, """
        2026-01-03 Salary
            Assets:Checking  100
            Income:Salary

        ~ fortnightly from 2026-01-01 Pay
            Assets:Checking  100
            Income:Salary
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines()[0] == f"{path}:5: unknown period 'fortnightly'"
    refused = tally("forecast", path, "--until", "2026-02-01")
    assert refused.returncode == 2


def test_malformed_header_and_bad_date_reported(tmp_path):
    path = write(tmp_path, """
        ~ monthly 2026-01-01 Rent
            Expenses:Rent  1200
            Assets:Checking

        ~ monthly from 2026-02-30 Rent
            Expenses:Rent  1200
            Assets:Checking
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines()[:2] == [
        f"{path}:1: invalid periodic header",
        f"{path}:5: invalid date '2026-02-30'",
    ]


def test_unbalanced_periodic_transaction_reported(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-01 Rent
            Expenses:Rent  1200
            Assets:Checking  -1100
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines()[0] == f"{path}:1: transaction does not balance (off by 100.00)"


def test_until_is_required(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-01 Rent
            Expenses:Rent  1200
            Assets:Checking
        """)
    result = tally("forecast", path)
    assert result.returncode == 2
