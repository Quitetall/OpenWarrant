"""Hidden acceptance tests for T2: balance --convert COMMODITY [--at DATE]."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t2.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


JOURNAL = """
P 2026-01-15 EUR 1.0850 USD
P 2026-02-01 EUR 1.1000 USD
P 2026-01-01 GBP 1.25 USD

2026-01-10 Opening
    Assets:Wallet  200.00 EUR
    Assets:Bank  1000.00 USD
    Assets:Pounds  40 GBP
    Equity:Opening  -200.00 EUR
    Equity:Opening  -1000.00 USD
    Equity:Opening  -40 GBP

2026-01-20 Cafe
    Expenses:Food  12.34 EUR
    Assets:Wallet
"""


def test_convert_at_a_date_uses_latest_price_on_or_before_it(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--convert", "USD", "--at", "2026-01-31")
    assert result.returncode == 0, result.stderr
    # Wallet: 200.00 * 1.0850 = 217.00 and -12.34 * 1.0850 = -13.3889 -> -13.39, so 203.61.
    # Food: 12.34 * 1.0850 = 13.3889 -> 13.39.
    # Equity: -1000 + (-200 * 1.0850 = -217.00) + (-40 * 1.25 = -50.00) = -1267.00
    assert result.stdout == (
        "   1000.00 USD  Assets:Bank\n"
        "     50.00 USD  Assets:Pounds\n"
        "    203.61 USD  Assets:Wallet\n"
        "  -1267.00 USD  Equity:Opening\n"
        "     13.39 USD  Expenses:Food\n"
        "--------------\n"
        "      0.00 USD\n"
    )


def test_price_dated_on_the_at_date_is_used(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "Expenses", "--convert", "USD", "--at", "2026-02-01")
    assert result.returncode == 0, result.stderr
    # 12.34 * 1.1000 = 13.574 -> 13.57
    assert result.stdout.splitlines()[0] == "     13.57 USD  Expenses:Food"


def test_default_at_is_the_latest_transaction_or_price_date(tmp_path):
    # The latest date in the journal is the 2026-02-01 price, after the last transaction.
    result = tally("balance", write(tmp_path, JOURNAL), "Expenses", "--convert", "USD")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[0] == "     13.57 USD  Expenses:Food"


def test_same_day_prices_last_in_file_wins(tmp_path):
    path = write(tmp_path, """
        P 2026-01-15 EUR 2.00 USD
        P 2026-01-15 EUR 3.00 USD
        2026-01-20 Cafe
            Expenses:Food  10 EUR
            Assets:Wallet
        """)
    result = tally("balance", path, "Expenses", "--convert", "USD", "--at", "2026-01-20")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[0] == "     30.00 USD  Expenses:Food"


def test_each_posting_rounded_half_even_before_summing(tmp_path):
    # Each posting: 1.00 * 1.005 = 1.005 -> 1.00 (half to even), so three make 3.00.
    # Rounding the total instead would give 3.015 -> 3.02; rounding half up would give 3.03.
    path = write(tmp_path, """
        P 2026-01-01 EUR 1.005 USD
        2026-01-02 A
            Expenses:Fees  1.00 EUR
            Assets:Wallet
        2026-01-03 B
            Expenses:Fees  1.00 EUR
            Assets:Wallet
        2026-01-04 C
            Expenses:Fees  1.00 EUR
            Assets:Wallet
        """)
    result = tally("balance", path, "Expenses", "--convert", "USD")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "      3.00 USD  Expenses:Fees",
        "--------------",
        "      3.00 USD",
    ]


def test_default_commodity_amounts_are_left_unconverted(tmp_path):
    path = write(tmp_path, """
        P 2026-01-01 EUR 2 USD
        2026-01-02 A
            Assets:Cash  5
            Assets:Cash  4 EUR
            Equity:Opening  -5
            Equity:Opening  -4 EUR
        """)
    result = tally("balance", path, "Assets", "--convert", "USD")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "          5.00  Assets:Cash",
        "      8.00 USD  Assets:Cash",
        "--------------",
        "          5.00",
        "      8.00 USD",
    ]


def test_missing_price_exits_3_with_the_stated_message(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--convert", "USD", "--at", "2026-01-12")
    assert result.returncode == 3
    assert result.stderr == "tally: error: no price for EUR in USD on or before 2026-01-12\n"
    assert result.stdout == ""


def test_inverse_prices_are_not_used(tmp_path):
    path = write(tmp_path, """
        P 2026-01-01 USD 0.92 EUR
        2026-01-02 A
            Expenses:Food  10 EUR
            Assets:Wallet
        """)
    result = tally("balance", path, "--convert", "USD")
    assert result.returncode == 3
    assert result.stderr == "tally: error: no price for EUR in USD on or before 2026-01-02\n"


def test_only_postings_in_the_report_need_a_price(tmp_path):
    path = write(tmp_path, """
        2026-01-02 A
            Expenses:Food  10 EUR
            Assets:Wallet
        2026-01-03 B
            Expenses:Rent  50 USD
            Assets:Bank
        """)
    result = tally("balance", path, "Expenses:Rent", "--convert", "USD")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[0] == "     50.00 USD  Expenses:Rent"


def test_at_without_convert_is_a_usage_error(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--at", "2026-01-31")
    assert result.returncode == 2
    assert "--at requires --convert" in result.stderr


def test_invalid_at_date_is_a_usage_error(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--convert", "USD", "--at", "2026-02-30")
    assert result.returncode == 2
    assert "invalid date '2026-02-30'" in result.stderr


def test_invalid_target_commodity_is_a_usage_error(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--convert", "usd")
    assert result.returncode == 2
    assert "invalid commodity 'usd'" in result.stderr
