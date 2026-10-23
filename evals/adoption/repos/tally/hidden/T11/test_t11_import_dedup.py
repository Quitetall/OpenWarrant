"""Hidden acceptance tests for T11: import's commodity directive and --journal duplicates (within 3 days)."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def write(tmp_path, name, text):
    path = tmp_path / name
    path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
    return path


RULES = """
account Assets:Checking
skip 1
date-column 1
payee-column 2
amount-column 3
rule coffee => Expenses:Coffee
"""

CSV = """
Date,Description,Amount
2026-03-01,Blue Bottle Coffee,-4.50
2026-03-10,Employer,"1,000.00"
"""


def run_import(tmp_path, rules=RULES, csv=CSV, journal=None):
    args = ["import", write(tmp_path, "bank.csv", csv), "--rules", write(tmp_path, "bank.rules", rules)]
    if journal is not None:
        args += ["--journal", write(tmp_path, "main.ledger", journal)]
    return tally(*args)


def test_commodity_directive_tags_both_postings(tmp_path):
    result = run_import(tmp_path, rules=RULES + "commodity EUR\n")
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-03-01 Blue Bottle Coffee\n"
        "    Expenses:Coffee  4.50 EUR\n"
        "    Assets:Checking  -4.50 EUR\n"
        "\n"
        "2026-03-10 Employer\n"
        "    Expenses:Unknown  -1000.00 EUR\n"
        "    Assets:Checking  1000.00 EUR\n"
    )
    assert result.stderr == ""


def test_invalid_commodity_directive(tmp_path):
    result = run_import(tmp_path, rules=RULES + "commodity eur\n")
    assert result.returncode == 2
    assert result.stdout == ""
    assert result.stderr == f"tally: error: {tmp_path / 'bank.rules'}:7: invalid commodity 'eur'\n"


def test_row_matching_a_posting_two_days_earlier_is_skipped(tmp_path):
    journal = """
        2026-02-27 Coffee
            Expenses:Coffee  4.5
            Assets:Checking  -4.5
        """
    result = run_import(tmp_path, journal=journal)
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-03-10 Employer\n"
        "    Expenses:Unknown  -1000.00\n"
        "    Assets:Checking  1000.00\n"
    )
    assert result.stderr == "tally: note: skipped 1 duplicate row(s)\n"


def test_window_is_three_days_inclusive(tmp_path):
    journal = """
        2026-03-13 Salary
            Assets:Checking  1000
            Income:Salary

        2026-03-05 Coffee
            Expenses:Coffee  4.50
            Assets:Checking
        """
    # Salary is 3 days after its row: a duplicate. Coffee is 4 days after: not one.
    result = run_import(tmp_path, journal=journal)
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[0] == "2026-03-01 Blue Bottle Coffee"
    assert len(result.stdout.splitlines()) == 3
    assert result.stderr == "tally: note: skipped 1 duplicate row(s)\n"


def test_each_journal_posting_is_used_once_in_csv_order(tmp_path):
    csv = """
        Date,Description,Amount
        2026-03-01,Coffee A,-4.50
        2026-03-03,Coffee B,-4.50
        """
    journal = """
        2026-03-03 Coffee
            Expenses:Coffee  4.50
            Assets:Checking
        """
    # The first row is within the window and takes the only posting.
    result = run_import(tmp_path, csv=csv, journal=journal)
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines()[0] == "2026-03-03 Coffee B"
    assert result.stderr == "tally: note: skipped 1 duplicate row(s)\n"


def test_a_row_uses_the_closest_posting(tmp_path):
    csv = """
        Date,Description,Amount
        2026-03-04,Coffee A,-4.50
        2026-03-01,Coffee B,-4.50
        """
    journal = """
        2026-03-01 Coffee
            Expenses:Coffee  4.50
            Assets:Checking

        2026-03-05 Coffee
            Expenses:Coffee  4.50
            Assets:Checking
        """
    # Row A takes the 03-05 posting (1 day), leaving 03-01 for row B.
    result = run_import(tmp_path, csv=csv, journal=journal)
    assert result.returncode == 0, result.stderr
    assert result.stdout == ""
    assert result.stderr == "tally: note: skipped 2 duplicate row(s)\n"


def test_account_amount_and_commodity_must_match(tmp_path):
    journal = """
        2026-03-01 Coffee elsewhere
            Assets:Checking:Sub  -4.50
            Expenses:Coffee

        2026-03-01 Different amount
            Assets:Checking  -4.40
            Expenses:Coffee

        2026-03-10 Salary in euros
            Assets:Checking  1000 EUR
            Income:Salary
        """
    result = run_import(tmp_path, journal=journal)
    assert result.returncode == 0, result.stderr
    assert len(result.stdout.splitlines()) == 7
    assert result.stderr == ""


def test_duplicates_with_a_commodity(tmp_path):
    journal = """
        2026-03-11 Salary in euros
            Assets:Checking  1000.00 EUR
            Income:Salary
        """
    result = run_import(tmp_path, rules=RULES + "commodity EUR\n", journal=journal)
    assert result.returncode == 0, result.stderr
    assert result.stdout.splitlines() == [
        "2026-03-01 Blue Bottle Coffee",
        "    Expenses:Coffee  4.50 EUR",
        "    Assets:Checking  -4.50 EUR",
    ]
    assert result.stderr == "tally: note: skipped 1 duplicate row(s)\n"


def test_periodic_transactions_are_not_matched(tmp_path):
    journal = """
        ~ monthly from 2026-01-01 Salary
            Assets:Checking  1000
            Income:Salary
        """
    result = run_import(tmp_path, csv=CSV.replace("2026-03-10", "2026-03-01", 1).replace("2026-03-01,Blue", "2026-03-02,Blue"), journal=journal)
    assert result.returncode == 0, result.stderr
    assert result.stderr == ""
    assert len(result.stdout.splitlines()) == 7


def test_journal_with_problems_is_refused(tmp_path):
    journal = """
        2026-03-01 Broken
            Assets:Checking  -4.50
            Expenses:Coffee  4.00
        """
    result = run_import(tmp_path, journal=journal)
    assert result.returncode == 2
    assert result.stdout == ""
    assert result.stderr == (
        f"tally: error: {tmp_path / 'main.ledger'}:1: transaction does not balance (off by -0.50)\n"
    )


def test_missing_journal_file_is_refused(tmp_path):
    result = tally(
        "import", write(tmp_path, "bank.csv", CSV), "--rules", write(tmp_path, "bank.rules", RULES),
        "--journal", tmp_path / "nope.ledger",
    )
    assert result.returncode == 2
    assert result.stdout == ""
    assert result.stderr.startswith("tally: error: ")
