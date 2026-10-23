"""Hidden acceptance tests for T10: @ costs (the total for the quantity) and --cost."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t10.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


JOURNAL = """
P 2026-03-01 USD 0.90 EUR
P 2026-03-01 AAPL 200 USD

2026-01-02 Opening
    Assets:Checking  5000.00 USD
    Equity:Opening

2026-02-01 Buy shares
    Assets:Broker:AAPL  10 AAPL @ 1500.00 USD
    Assets:Checking

2026-02-15 Sell some
    Assets:Broker:AAPL  -4 AAPL @ 700 USD
    Assets:Checking  700.00 USD

2026-02-20 Buy euros
    Assets:Broker:Cash  100 EUR @ 110.00 USD
    Assets:Checking  -110.00 USD
"""


def test_an_elided_posting_receives_the_negated_cost(tmp_path):
    path = write(tmp_path, JOURNAL)
    result = tally("print", path, "--from", "2026-02-01", "--until", "2026-02-01")
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-02-01 Buy shares\n"
        "    Assets:Broker:AAPL  10.00 AAPL @ 1500.00 USD\n"
        "    Assets:Checking  -1500.00 USD\n"
    )
    check = tally("check", path)
    assert (check.returncode, check.stdout) == (0, "ok: 4 transactions, 8 postings\n")


def test_print_output_is_a_journal_check_accepts(tmp_path):
    printed = tally("print", write(tmp_path, JOURNAL))
    assert printed.returncode == 0, printed.stderr
    assert "    Assets:Broker:AAPL  -4.00 AAPL @ 700.00 USD\n" in printed.stdout
    again = tally("check", write(tmp_path, printed.stdout, "again.ledger"))
    assert again.returncode == 0, again.stdout


def test_balancing_uses_the_weights(tmp_path):
    path = write(tmp_path, """
        2026-02-01 Short
            Assets:Broker  10 AAPL @ 1500 USD
            Assets:Checking  -1400 USD

        2026-02-02 Fine
            Assets:Broker  -2 AAPL @ 320 USD
            Assets:Checking  320 USD
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout == (
        f"{path}:1: transaction does not balance in USD (off by 100.00)\n"
        "1 problem(s) found\n"
    )


def test_cost_problems(tmp_path):
    path = write(tmp_path, """
        2026-01-01 Zero
            Assets:A  1 AAPL @ 0 USD
            Assets:B

        2026-01-02 Negative
            Assets:A  1 AAPL @ -5 USD
            Assets:B

        2026-01-03 Words
            Assets:A  1 AAPL @ lots of   money
            Assets:B

        2026-01-04 Bad code
            Assets:A  1 AAPL @ 5 usd
            Assets:B

        2026-01-05 Same
            Assets:A  1 AAPL @ 5 AAPL
            Assets:B

        2026-01-06 Both default
            Assets:A  1 @ 5
            Assets:B
        """)
    result = tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines() == [
        f"{path}:2: invalid cost '0 USD'",
        f"{path}:6: invalid cost '-5 USD'",
        f"{path}:10: invalid cost 'lots of money'",
        f"{path}:14: invalid cost '5 usd'",
        f"{path}:18: cost must be in a different commodity",
        f"{path}:22: cost must be in a different commodity",
        "6 problem(s) found",
    ]
    assert tally("balance", path).returncode == 2


def test_balance_shows_quantities_and_cost_shows_weights(tmp_path):
    path = write(tmp_path, JOURNAL)
    plain = tally("balance", path, "Assets:Broker")
    assert plain.returncode == 0, plain.stderr
    assert plain.stdout.splitlines()[:2] == [
        "     6.00 AAPL  Assets:Broker:AAPL",
        "    100.00 EUR  Assets:Broker:Cash",
    ]
    cost = tally("balance", path, "--cost")
    assert cost.returncode == 0, cost.stderr
    assert cost.stdout == (
        "    800.00 USD  Assets:Broker:AAPL\n"
        "    110.00 USD  Assets:Broker:Cash\n"
        "   4090.00 USD  Assets:Checking\n"
        "  -5000.00 USD  Equity:Opening\n"
        "--------------\n"
        "      0.00 USD\n"
    )


def test_cost_then_convert_and_depth(tmp_path):
    path = write(tmp_path, JOURNAL)
    result = tally("balance", path, "Assets", "--cost", "--convert", "EUR", "--depth", "2")
    assert result.returncode == 0, result.stderr
    # Weights 800 USD + 110 USD, each converted at 0.90.
    assert result.stdout.splitlines()[:2] == [
        "    819.00 EUR  Assets:Broker",
        "   3681.00 EUR  Assets:Checking",
    ]


def test_a_cost_already_in_the_target_is_not_converted(tmp_path):
    path = write(tmp_path, """
        2026-03-01 Buy
            Assets:Broker  3 AAPL @ 541.00 EUR
            Assets:Cash
        """)
    result = tally("balance", path, "Assets:Broker", "--cost", "--convert", "EUR")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[0] == "    541.00 EUR  Assets:Broker"


def test_costs_are_not_prices(tmp_path):
    path = write(tmp_path, """
        2026-03-01 Buy
            Assets:Broker  3 AAPL @ 541.00 USD
            Assets:Cash
        """)
    prices = tally("prices", path)
    assert (prices.returncode, prices.stdout) == (0, "")
    convert = tally("balance", path, "Assets:Broker", "--convert", "USD")
    assert convert.returncode == 3
    assert convert.stderr == "tally: error: no price for AAPL in USD on or before 2026-03-01\n"


def test_register_cost(tmp_path):
    result = tally("register", write(tmp_path, JOURNAL), "Assets:Broker", "--cost")
    assert result.returncode == 0, result.stderr
    assert [line.split()[-4:] for line in result.stdout.splitlines()] == [
        ["1500.00", "USD", "1500.00", "USD"],
        ["-700.00", "USD", "800.00", "USD"],
        ["110.00", "USD", "910.00", "USD"],
    ]


def test_networth_cost(tmp_path):
    result = tally("networth", write(tmp_path, JOURNAL), "--until", "2026-02-28", "--cost")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        f"2026-01-31  {'5000.00 USD':>14}",
        f"2026-02-28  {'5000.00 USD':>14}",
    ]
    plain = tally("networth", write(tmp_path, JOURNAL), "--until", "2026-02-28")
    assert plain.stdout.splitlines()[1:] == [
        f"2026-02-28  {'6.00 AAPL':>14}",
        f"2026-02-28  {'100.00 EUR':>14}",
        f"2026-02-28  {'4090.00 USD':>14}",
    ]


def test_forecast_writes_costs(tmp_path):
    path = write(tmp_path, """
        ~ monthly from 2026-01-10 Savings plan
            Assets:Broker  2 VTI @ 450.00 USD
            Assets:Checking
        """)
    result = tally("forecast", path, "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-01-10 Savings plan\n"
        "    Assets:Broker  2.00 VTI @ 450.00 USD\n"
        "    Assets:Checking  -450.00 USD\n"
    )


def test_cost_in_the_default_commodity(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Buy
            Assets:Broker  2 AAPL @ 300
            Assets:Checking
        """)
    result = tally("print", path)
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[1:] == [
        "    Assets:Broker  2.00 AAPL @ 300.00",
        "    Assets:Checking  -300.00",
    ]
