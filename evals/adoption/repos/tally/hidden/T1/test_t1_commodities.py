"""Hidden acceptance tests for T1: commodities and price directives."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t1.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


MIXED = """
2026-01-02 Exchange
    Assets:Wallet  100.00 EUR
    Equity:Conversion

2026-01-03 Salary
    Assets:Bank  500
    Income:Salary

2026-01-04 Shares
    Assets:Broker  3 AAPL
    Equity:Conversion  -3 AAPL

2026-01-05 Cafe
    Expenses:Food  4.20 EUR
    Assets:Wallet
"""


def test_balance_one_row_per_account_and_commodity(tmp_path):
    result = tally("balance", write(tmp_path, MIXED))
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "        500.00  Assets:Bank\n"
        "     3.00 AAPL  Assets:Broker\n"
        "     95.80 EUR  Assets:Wallet\n"
        "    -3.00 AAPL  Equity:Conversion\n"
        "   -100.00 EUR  Equity:Conversion\n"
        "      4.20 EUR  Expenses:Food\n"
        "       -500.00  Income:Salary\n"
        "--------------\n"
        "          0.00\n"
        "     0.00 AAPL\n"
        "      0.00 EUR\n"
    )


def test_default_commodity_sorts_first_within_an_account(tmp_path):
    path = write(tmp_path, """
        2026-01-02 A
            Assets:Cash  10 USD
            Equity:Opening
        2026-01-03 B
            Assets:Cash  7
            Equity:Opening
        """)
    result = tally("balance", path, "Assets")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "          7.00  Assets:Cash",
        "     10.00 USD  Assets:Cash",
        "--------------",
        "          7.00",
        "     10.00 USD",
    ]


def test_elided_amount_takes_the_other_postings_commodity(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Cafe
            Expenses:Food  42.50 EUR
            Assets:Wallet
        """)
    result = tally("print", path)
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-01-05 Cafe\n"
        "    Expenses:Food  42.50 EUR\n"
        "    Assets:Wallet  -42.50 EUR\n"
    )


def test_check_balances_each_commodity_separately(tmp_path):
    path = write(tmp_path, """
        ; header comment
        2026-01-05 Mismatch
            Assets:Wallet  10 EUR
            Assets:Bank  -10 USD
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert f"{path}:2: transaction does not balance in EUR (off by 10.00)" in result.stdout.splitlines()
    assert f"{path}:2: transaction does not balance in USD (off by -10.00)" in result.stdout.splitlines()


def test_default_commodity_keeps_the_existing_message(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Off
            Assets:Wallet  10 EUR
            Assets:Bank  -10 EUR
            Expenses:Fees  2.5
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines()[0] == f"{path}:1: transaction does not balance (off by 2.50)"


def test_elision_refused_when_postings_use_two_commodities(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Ambiguous
            Assets:Wallet  10 EUR
            Assets:Bank  5 USD
            Equity:Conversion
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines()[0] == (
        f"{path}:4: cannot infer amount: postings use more than one commodity"
    )
    refused = tally("balance", path)
    assert refused.returncode == 2
    assert refused.stdout == ""


def test_invalid_commodity_code_reported(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Lower
            Assets:Wallet  10 eur
            Equity:Opening
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines()[0] == f"{path}:2: invalid commodity 'eur'"


def test_register_keeps_a_running_total_per_commodity(tmp_path):
    path = write(tmp_path, """
        2026-01-02 Open EUR
            Assets:Wallet  100 EUR
            Equity:Opening
        2026-01-03 Open USD
            Assets:Wallet  50 USD
            Equity:Opening
        2026-01-04 Spend
            Assets:Wallet  -30 EUR
            Expenses:Food
        """)
    result = tally("register", path, "Assets:Wallet")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "2026-01-02 Open EUR             Assets:Wallet                100.00 EUR     100.00 EUR",
        "2026-01-03 Open USD             Assets:Wallet                 50.00 USD      50.00 USD",
        "2026-01-04 Spend                Assets:Wallet                -30.00 EUR      70.00 EUR",
    ]


PRICES = """
P 2026-03-01 EUR 1.0900 USD
P 2026-01-15 EUR 1.0850 USD
P 2026-01-15 GBP 1.27 USD

2026-01-20 Something
    Assets:Wallet  1 EUR
    Equity:Opening

P 2026-02-01 EUR 1.1 USD   ; trailing comment
"""


def test_prices_lists_directives_by_date_with_rate_as_written(tmp_path):
    result = tally("prices", write(tmp_path, PRICES))
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "2026-01-15 EUR 1.0850 USD",
        "2026-01-15 GBP 1.27 USD",
        "2026-02-01 EUR 1.1 USD",
        "2026-03-01 EUR 1.0900 USD",
    ]


def test_prices_filtered_by_commodity(tmp_path):
    result = tally("prices", write(tmp_path, PRICES), "GBP")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == ["2026-01-15 GBP 1.27 USD"]


def test_bad_price_directives_reported_with_file_and_line(tmp_path):
    path = write(tmp_path, """
        P 2026-01-15 EUR 1.0850
        P 2026-02-30 EUR 1.0850 USD
        P 2026-01-15 eur 1.0850 USD
        P 2026-01-15 EUR -1 USD
        P 2026-01-15 EUR abc USD
        P 2026-01-15 EUR 1.0850 USD
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines()[:5] == [
        f"{path}:1: invalid price directive",
        f"{path}:2: invalid date '2026-02-30'",
        f"{path}:3: invalid commodity 'eur'",
        f"{path}:4: invalid price '-1'",
        f"{path}:5: invalid price 'abc'",
    ]
    refused = tally("prices", path)
    assert refused.returncode == 2
