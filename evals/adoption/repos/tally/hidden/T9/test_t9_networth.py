"""Hidden acceptance tests for T9: networth, with T1 commodities, T2 conversion and T3 forecasts."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t9.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


def line(date, amount):
    return f"{date}  {amount:>14}"


JOURNAL = """
2026-01-14 Salary
    Assets:Checking  2000.00
    Income:Salary

2026-01-20 Groceries
    Expenses:Food  80
    Liabilities:Card:Visa

2026-02-03 Holiday cash
    Assets:Wallet  100 EUR
    Equity:Exchange  -100 EUR

2026-03-02 Pay card
    Liabilities:Card:Visa  80
    Assets:Checking

2026-03-05 Old account
    AssetsOld:Box  500
    Income:Gift
"""


def test_month_ends_from_the_first_transaction_through_until(tmp_path):
    result = tally("networth", write(tmp_path, JOURNAL), "--until", "2026-03-15")
    assert result.returncode == 0, result.stderr
    assert result.stdout == "\n".join([
        line("2026-01-31", "1920.00"),
        line("2026-02-28", "1920.00"),
        line("2026-02-28", "100.00 EUR"),
        line("2026-03-15", "1920.00"),
        line("2026-03-15", "100.00 EUR"),
    ]) + "\n"


def test_until_on_a_month_end_and_from_only_moves_the_start(tmp_path):
    result = tally("networth", write(tmp_path, JOURNAL), "--from", "2026-02-10", "--until", "2026-02-28")
    assert result.returncode == 0, result.stderr
    # Everything up to the row date counts, including January's postings.
    assert result.stdout.splitlines() == [line("2026-02-28", "1920.00"), line("2026-02-28", "100.00 EUR")]


def test_rows_before_any_counted_posting_are_zero(tmp_path):
    result = tally("networth", write(tmp_path, JOURNAL), "--from", "2025-11-20", "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        line("2025-11-30", "0.00"),
        line("2025-12-31", "0.00"),
        line("2026-01-31", "1920.00"),
    ]


def test_a_commodity_that_nets_to_zero_keeps_its_line(tmp_path):
    path = write(tmp_path, """
        2026-01-05 In
            Assets:Wallet  50 EUR
            Equity:Opening
        2026-01-06 Out
            Expenses:Food  50 EUR
            Assets:Wallet
        """)
    result = tally("networth", path, "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [line("2026-01-31", "0.00 EUR")]


PRICED = """
P 2026-01-31 EUR 1.005 USD
P 2026-02-15 EUR 2.00 USD
P 2026-02-15 EUR 1.50 USD
P 2026-03-31 EUR 9.00 USD

2026-01-10 Coins
    Assets:Jar  1.00 EUR
    Assets:Jar  1.00 EUR
    Assets:Jar  1.00 EUR
    Equity:Opening  -3.00 EUR

2026-01-11 Dust
    Assets:Dust  0.005 USD
    Assets:Dust  0.005 USD
    Assets:Dust  0.005 USD
    Income:Dust  -0.015 USD

2026-01-12 Plain
    Assets:Plain  7
    Income:Plain

2026-01-13 Trip
    Expenses:Travel  40 GBP
    Equity:Opening  -40 GBP
"""


def test_convert_each_row_at_its_own_date_by_balance_convert_rules(tmp_path):
    result = tally("networth", write(tmp_path, PRICED), "--until", "2026-02-28", "--convert", "USD")
    assert result.returncode == 0, result.stderr
    # Jan 31: three EUR postings at 1.005, each rounded half-even (1.00) = 3.00,
    # plus 0.005 * 3 USD summed unrounded = 0.015 -> 0.02. The default commodity stays.
    # Feb 28: the later of the two Feb 15 prices, 1.50: 4.50 + 0.015 -> 4.52.
    # The GBP posting is not counted, so it needs no price.
    assert result.stdout.splitlines() == [
        line("2026-01-31", "7.00"),
        line("2026-01-31", "3.02 USD"),
        line("2026-02-28", "7.00"),
        line("2026-02-28", "4.52 USD"),
    ]


def test_missing_price_on_a_row_date_fails_like_balance_convert(tmp_path):
    # The only EUR price is dated 2026-01-31, after the row date 2026-01-30.
    result = tally("networth", write(tmp_path, PRICED), "--until", "2026-01-30", "--convert", "USD")
    assert result.returncode == 3
    assert result.stdout == ""
    assert result.stderr == "tally: error: no price for EUR in USD on or before 2026-01-30\n"


def test_missing_price_names_the_first_failing_row(tmp_path):
    path = write(tmp_path, """
        P 2026-01-31 EUR 1.10 USD
        2026-01-10 Coins
            Assets:Jar  10 EUR
            Equity:Opening
        2026-02-10 Pounds
            Assets:Jar  10 GBP
            Equity:Opening
        """)
    result = tally("networth", path, "--until", "2026-03-31", "--convert", "USD")
    assert result.returncode == 3
    assert result.stdout == ""
    assert result.stderr == "tally: error: no price for GBP in USD on or before 2026-02-28\n"


FORECAST = """
P 2026-01-01 EUR 1.10 USD
P 2026-03-01 EUR 1.20 USD

2026-01-05 Opening
    Assets:Checking  1000.00 USD
    Equity:Opening

2026-01-20 Spend
    Expenses:Food  25 EUR
    Liabilities:Card

~ monthly from 2026-01-15 Allowance
    Assets:Wallet  100 EUR
    Income:Allowance

~ monthly from 2026-02-01 Rent
    Expenses:Rent  300.00 USD
    Assets:Checking
"""


def test_forecast_counts_occurrences_after_the_last_real_transaction(tmp_path):
    result = tally("networth", write(tmp_path, FORECAST), "--until", "2026-03-10", "--forecast")
    assert result.returncode == 0, result.stderr
    # Last real transaction: 2026-01-20, so the Jan 15 allowance does not count.
    assert result.stdout.splitlines() == [
        line("2026-01-31", "-25.00 EUR"),
        line("2026-01-31", "1000.00 USD"),
        line("2026-02-28", "75.00 EUR"),
        line("2026-02-28", "700.00 USD"),
        line("2026-03-10", "75.00 EUR"),
        line("2026-03-10", "400.00 USD"),
    ]


def test_forecast_with_convert(tmp_path):
    result = tally("networth", write(tmp_path, FORECAST), "--until", "2026-03-31", "--forecast", "--convert", "USD")
    assert result.returncode == 0, result.stderr
    # Jan: 1000 - 27.50. Feb: 700 - 27.50 + 110.00. Mar: 400 - 30.00 + 240.00.
    assert result.stdout.splitlines() == [
        line("2026-01-31", "972.50 USD"),
        line("2026-02-28", "782.50 USD"),
        line("2026-03-31", "610.00 USD"),
    ]


def test_without_forecast_periodic_transactions_do_not_count(tmp_path):
    result = tally("networth", write(tmp_path, FORECAST), "--until", "2026-02-28", "--convert", "USD")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [line("2026-01-31", "972.50 USD"), line("2026-02-28", "972.50 USD")]


def test_forecast_with_no_real_transactions(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-31 Savings
            Assets:Savings  50
            Assets:Checking  -20
            Income:Salary
        """)
    result = tally("networth", path, "--until", "2026-03-31", "--forecast")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        line("2026-03-31", "90.00"),
    ]


def test_usage_errors(tmp_path):
    path = write(tmp_path, JOURNAL)
    swapped = tally("networth", path, "--from", "2026-03-01", "--until", "2026-01-31")
    assert (swapped.returncode, swapped.stdout) == (2, "")
    assert swapped.stderr == "tally: error: --from must not be after --until\n"
    bad = tally("networth", path, "--until", "2026-03-31", "--convert", "usd")
    assert (bad.returncode, bad.stdout) == (2, "")
    assert "invalid commodity 'usd'" in bad.stderr
    missing = tally("networth", path)
    assert (missing.returncode, missing.stdout) == (2, "")


def test_journal_with_problems_is_refused(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Broken
            Assets:Checking  10 EUR
            Equity:Opening  -9 EUR
        """)
    result = tally("networth", path, "--until", "2026-01-31")
    assert (result.returncode, result.stdout) == (2, "")
    assert "does not balance in EUR (off by 1.00)" in result.stderr
