"""Hidden acceptance tests for T6: `balance --depth N`."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, text, name="t6.ledger"):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


JOURNAL = """
2026-01-01 Salary
    Assets:Bank:Checking  3000
    Income:Salary

2026-01-02 Transfer
    Assets:Bank:Savings  500
    Assets:Bank:Checking

2026-01-03 Market
    Expenses:Food:Groceries  120.50
    Assets:Cash

2026-01-04 Restaurant
    Expenses:Food:Dining  45.25
    Assets:Bank:Checking

2026-01-05 Bus
    Expenses:Transport  2.75
    Assets:Cash

2026-01-06 Gift
    Expenses  10
    Assets:Cash
"""


def test_depth_one_rolls_everything_into_top_level_accounts(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--depth", "1")
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "       2821.50  Assets\n"
        "        178.50  Expenses\n"
        "      -3000.00  Income\n"
        "--------------\n"
        "          0.00\n"
    )


def test_depth_two_keeps_shallower_accounts_unchanged(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--depth", "2")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "       2954.75  Assets:Bank",
        "       -133.25  Assets:Cash",
        "         10.00  Expenses",
        "        165.75  Expenses:Food",
        "          2.75  Expenses:Transport",
        "      -3000.00  Income:Salary",
        "--------------",
        "          0.00",
    ]


def test_depth_deeper_than_any_account_changes_nothing(tmp_path):
    path = write(tmp_path, JOURNAL)
    plain = tally("balance", path)
    deep = tally("balance", path, "--depth", "9")
    assert deep.returncode == 0, deep.stderr
    assert deep.stdout == plain.stdout


def test_depth_applies_after_the_account_filter(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "Expenses:Food", "--depth", "2")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "        165.75  Expenses:Food",
        "--------------",
        "        165.75",
    ]


def test_depth_combines_with_date_range(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--depth", "1", "--from", "2026-01-03", "--until", "2026-01-04")
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[:2] == [
        "       -165.75  Assets",
        "        165.75  Expenses",
    ]


def test_depth_zero_refused(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--depth", "0")
    assert result.returncode == 2
    assert result.stderr == "tally: error: --depth must be at least 1\n"
    assert result.stdout == ""


def test_negative_depth_refused(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--depth", "-2")
    assert result.returncode == 2
    assert "--depth must be at least 1" in result.stderr


def test_non_integer_depth_refused(tmp_path):
    result = tally("balance", write(tmp_path, JOURNAL), "--depth", "two")
    assert result.returncode == 2
    assert result.stdout == ""
