"""Hidden acceptance tests for T7: export, with the separator and decimal mark from the answer."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t7.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


JOURNAL = """
2026-01-09 Later
    Expenses:Food:Groceries  12345.6
    Assets:Checking

2026-01-05 Corner Grocery
    Expenses:Food  42.50
    Assets:Checking

2026-01-05 Exchange
    Assets:Wallet  100 EUR
    Equity:Exchange  -100 EUR

~ monthly from 2026-01-01 Rent
    Expenses:Rent  1200
    Assets:Checking
"""


def test_header_and_rows_in_print_order(tmp_path):
    result = tally("export", write(tmp_path, JOURNAL))
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "date;payee;account;amount;commodity\n"
        "2026-01-05;Corner Grocery;Expenses:Food;42,50;\n"
        "2026-01-05;Corner Grocery;Assets:Checking;-42,50;\n"
        "2026-01-05;Exchange;Assets:Wallet;100,00;EUR\n"
        "2026-01-05;Exchange;Equity:Exchange;-100,00;EUR\n"
        "2026-01-09;Later;Expenses:Food:Groceries;12345,60;\n"
        "2026-01-09;Later;Assets:Checking;-12345,60;\n"
    )


def test_rows_end_with_a_bare_newline(tmp_path):
    result = subprocess.run(
        [sys.executable, "-m", "tally", "export", str(write(tmp_path, JOURNAL))],
        cwd=os.getcwd(), capture_output=True, timeout=60,
    )
    assert result.returncode == 0
    assert b"\r" not in result.stdout
    assert result.stdout.count(b"\n") == 7


def test_amounts_round_half_even_and_zero_has_no_sign(tmp_path):
    path = write(tmp_path, """
        2026-02-01 Odd cents
            Assets:A  2.675
            Assets:B  0.125
            Assets:C  -0.005
            Assets:D  -2.795
        """)
    result = tally("export", path)
    assert result.returncode == 0, result.stderr
    assert [line.split(";")[3] for line in result.stdout.splitlines()[1:]] == [
        "2,68", "0,12", "0,00", "-2,80",
    ]


def test_quoting_only_for_separator_and_double_quote(tmp_path):
    path = write(tmp_path, """
        2026-03-01 Smith, Jones & Co
            Expenses:Legal  10
            Assets:Checking

        2026-03-02 Bar; Grill
            Expenses:Food  20
            Assets:Checking

        2026-03-03 Cafe "Zum"
            Expenses:Food  5 EUR
            Assets:Wallet
        """)
    result = tally("export", path, "Expenses")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[1:] == [
        "2026-03-01;Smith, Jones & Co;Expenses:Legal;10,00;",
        '2026-03-02;"Bar; Grill";Expenses:Food;20,00;',
        '2026-03-03;"Cafe ""Zum""";Expenses:Food;5,00;EUR',
    ]


def test_empty_payee_is_an_empty_field(tmp_path):
    path = write(tmp_path, """
        2026-03-01
            Assets:A  1
            Assets:B
        """)
    result = tally("export", path)
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[1:] == [
        "2026-03-01;;Assets:A;1,00;",
        "2026-03-01;;Assets:B;-1,00;",
    ]


def test_account_argument_keeps_descendants(tmp_path):
    result = tally("export", write(tmp_path, JOURNAL), "Expenses:Food")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "date;payee;account;amount;commodity",
        "2026-01-05;Corner Grocery;Expenses:Food;42,50;",
        "2026-01-09;Later;Expenses:Food:Groceries;12345,60;",
    ]


def test_date_range_selects_transactions(tmp_path):
    result = tally("export", write(tmp_path, JOURNAL), "--from", "2026-01-06", "--until", "2026-01-31")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "date;payee;account;amount;commodity",
        "2026-01-09;Later;Expenses:Food:Groceries;12345,60;",
        "2026-01-09;Later;Assets:Checking;-12345,60;",
    ]


def test_periodic_only_journal_prints_just_the_header(tmp_path):
    path = write(tmp_path, """
        ~ weekly from 2026-01-01 Allowance
            Expenses:Allowance  20
            Assets:Checking
        """)
    result = tally("export", path)
    assert result.returncode == 0, result.stderr
    assert result.stdout == "date;payee;account;amount;commodity\n"


def test_journal_with_problems_is_refused(tmp_path):
    path = write(tmp_path, """
        2026-01-05 Broken
            Expenses:Food  10
            Assets:Checking  -9
        """)
    result = tally("export", path)
    assert result.returncode == 2
    assert result.stdout == ""
    assert "does not balance (off by 1.00)" in result.stderr


def test_from_after_until_is_refused(tmp_path):
    result = tally("export", write(tmp_path, JOURNAL), "--from", "2026-02-01", "--until", "2026-01-01")
    assert result.returncode == 2
    assert result.stdout == ""
    assert result.stderr == "tally: error: --from must not be after --until\n"
