"""Hidden acceptance tests for T5: `import CSV --rules FILE`."""

import os
import subprocess
import sys
import textwrap


def tally(*args):
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=os.getcwd(), capture_output=True, text=True, timeout=60,
    )


def files(tmp_path, rules, csv_text):
    rules_path = tmp_path / "bank.rules"
    csv_path = tmp_path / "bank.csv"
    rules_path.write_text(textwrap.dedent(rules).lstrip("\n"), encoding="utf-8")
    csv_path.write_text(textwrap.dedent(csv_text).lstrip("\n"), encoding="utf-8")
    return csv_path, rules_path


RULES = """
# Checking account export
account Assets:Checking
skip 1
date-column 2
payee-column 3
amount-column 4
date-format %m/%d/%Y

rule ^blue bottle => Expenses:Coffee
rule coffee => Expenses:Coffee:Other
rule amazon|ebay => Expenses:Shopping
rule coffee => Expenses:Never
"""

CSV = """
Ref,Date,Description,Amount
1,03/01/2026,BLUE BOTTLE COFFEE #12,-4.50
2,03/02/2026,  Joe's Coffee  ,-3.25
3,03/03/2026,AMAZON MKTP,"-1,234.56"
4,03/04/2026,ACME PAYROLL,2500

5,03/05/2026,Bookshop,-20
"""


def test_import_exact_output(tmp_path):
    csv_path, rules_path = files(tmp_path, RULES, CSV)
    result = tally("import", csv_path, "--rules", rules_path)
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-03-01 BLUE BOTTLE COFFEE #12\n"
        "    Expenses:Coffee  4.50\n"
        "    Assets:Checking  -4.50\n"
        "\n"
        "2026-03-02 Joe's Coffee\n"
        "    Expenses:Coffee:Other  3.25\n"
        "    Assets:Checking  -3.25\n"
        "\n"
        "2026-03-03 AMAZON MKTP\n"
        "    Expenses:Shopping  1234.56\n"
        "    Assets:Checking  -1234.56\n"
        "\n"
        "2026-03-04 ACME PAYROLL\n"
        "    Expenses:Unknown  -2500.00\n"
        "    Assets:Checking  2500.00\n"
        "\n"
        "2026-03-05 Bookshop\n"
        "    Expenses:Unknown  20.00\n"
        "    Assets:Checking  -20.00\n"
    )


def test_defaults_iso_dates_no_skip_and_default_account(tmp_path):
    rules = """
        account Assets:Card
        date-column 1
        payee-column 2
        amount-column 3
        default-account Expenses:Misc
        """
    csv_text = """
        2026-04-01,Corner Shop,-7
        2026-04-02,Refund,7.10
        """
    csv_path, rules_path = files(tmp_path, rules, csv_text)
    result = tally("import", csv_path, "--rules", rules_path)
    assert result.returncode == 0, result.stderr
    assert result.stdout == (
        "2026-04-01 Corner Shop\n"
        "    Expenses:Misc  7.00\n"
        "    Assets:Card  -7.00\n"
        "\n"
        "2026-04-02 Refund\n"
        "    Expenses:Misc  -7.10\n"
        "    Assets:Card  7.10\n"
    )


def test_imported_output_is_a_valid_journal(tmp_path):
    csv_path, rules_path = files(tmp_path, RULES, CSV)
    result = tally("import", csv_path, "--rules", rules_path)
    journal = tmp_path / "imported.ledger"
    journal.write_text(result.stdout, encoding="utf-8")
    check = tally("check", journal)
    assert check.returncode == 0, check.stdout
    assert check.stdout == "ok: 5 transactions, 10 postings\n"


def test_missing_required_directive(tmp_path):
    rules = """
        account Assets:Checking
        date-column 1
        amount-column 3
        """
    csv_path, rules_path = files(tmp_path, rules, "2026-04-01,Shop,-7\n")
    result = tally("import", csv_path, "--rules", rules_path)
    assert result.returncode == 2
    assert result.stderr == f"tally: error: {rules_path}: missing 'payee-column'\n"
    assert result.stdout == ""


def test_unknown_directive_and_invalid_rule_lines(tmp_path):
    base = "account Assets:Checking\ndate-column 1\npayee-column 2\namount-column 3\n"
    cases = [
        ("currency USD\n", "unknown directive 'currency'"),
        ("rule coffee Expenses:Coffee\n", "invalid rule"),
        ("rule (unclosed => Expenses:Coffee\n", "invalid regex '(unclosed'"),
        ("date-column zero\n", "invalid number 'zero'"),
        ("date-column 0\n", "invalid number '0'"),
    ]
    for extra, message in cases:
        csv_path, rules_path = files(tmp_path, base + extra, "2026-04-01,Shop,-7\n")
        result = tally("import", csv_path, "--rules", rules_path)
        assert result.returncode == 2, extra
        assert result.stderr == f"tally: error: {rules_path}:5: {message}\n", extra
        assert result.stdout == ""


def test_bad_csv_rows_reported_with_line_and_no_output(tmp_path):
    rules = "account Assets:Checking\nskip 1\ndate-column 1\npayee-column 2\namount-column 3\n"
    cases = [
        ("Date,Payee,Amount\n2026-04-01,Shop,-7\n2026-13-01,Shop,-7\n", 3, "invalid date '2026-13-01'"),
        ("Date,Payee,Amount\n2026-04-01,Shop,-7\n2026-04-02,Shop,seven\n", 3, "invalid amount 'seven'"),
        ("Date,Payee,Amount\n2026-04-01,Shop\n", 2, "missing column 3"),
    ]
    for csv_text, line, message in cases:
        csv_path, rules_path = files(tmp_path, rules, csv_text)
        result = tally("import", csv_path, "--rules", rules_path)
        assert result.returncode == 2, csv_text
        assert result.stderr == f"tally: error: {csv_path}:{line}: {message}\n", csv_text
        assert result.stdout == ""


def test_rules_option_is_required(tmp_path):
    csv_path, _ = files(tmp_path, RULES, CSV)
    assert tally("import", csv_path).returncode == 2
