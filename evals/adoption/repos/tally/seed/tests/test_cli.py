from conftest import run_tally

GOOD = """
2026-01-01 Salary
    Assets:Checking  1000
    Income:Salary

2026-01-05 Grocery
    Expenses:Food  42.50
    Assets:Checking
"""


def test_balance_output(journal):
    result = run_tally("balance", journal(GOOD))
    assert result.returncode == 0
    assert result.stdout == (
        "        957.50  Assets:Checking\n"
        "         42.50  Expenses:Food\n"
        "      -1000.00  Income:Salary\n"
        "--------------\n"
        "          0.00\n"
    )


def test_register_output(journal):
    result = run_tally("register", journal(GOOD), "Assets")
    assert result.returncode == 0
    assert result.stdout.splitlines() == [
        "2026-01-01 Salary               Assets:Checking                 1000.00        1000.00",
        "2026-01-05 Grocery              Assets:Checking                  -42.50         957.50",
    ]


def test_check_ok(journal):
    result = run_tally("check", journal(GOOD))
    assert result.returncode == 0
    assert result.stdout == "ok: 2 transactions, 4 postings\n"


def test_check_reports_problems(journal):
    path = journal("2026-01-01 X\n    Assets:Cash  5\n    Income:Gift  -4\n")
    result = run_tally("check", path)
    assert result.returncode == 1
    assert result.stdout.splitlines() == [
        f"{path}:1: transaction does not balance (off by 1.00)",
        "1 problem(s) found",
    ]


def test_reports_refuse_a_broken_journal(journal):
    path = journal("2026-13-01 X\n    Assets:Cash  5\n    Income:Gift\n")
    result = run_tally("balance", path)
    assert result.returncode == 2
    assert result.stderr == f"tally: error: {path}:1: invalid date '2026-13-01'\n"
    assert result.stdout == ""


def test_missing_file(tmp_path):
    result = run_tally("balance", tmp_path / "nope.ledger")
    assert result.returncode == 2
    assert "nope.ledger" in result.stderr


def test_date_range_options(journal):
    result = run_tally("balance", journal(GOOD), "--from", "2026-01-02", "--until", "2026-01-31")
    assert result.returncode == 0
    assert result.stdout.splitlines()[:2] == [
        "        -42.50  Assets:Checking",
        "         42.50  Expenses:Food",
    ]


def test_empty_date_range_refused(journal):
    result = run_tally("register", journal(GOOD), "--from", "2026-02-01", "--until", "2026-01-01")
    assert result.returncode == 2
    assert result.stderr == "tally: error: --from must not be after --until\n"


def test_bad_date_option(journal):
    result = run_tally("balance", journal(GOOD), "--from", "2026-02-30")
    assert result.returncode == 2
    assert "invalid date '2026-02-30'" in result.stderr


def test_accounts(journal):
    result = run_tally("accounts", journal(GOOD))
    assert result.stdout.splitlines() == ["Assets:Checking", "Expenses:Food", "Income:Salary"]


def test_print_fills_amounts_and_sorts(journal):
    path = journal(
        """
        2026-01-05 Grocery   
            Expenses:Food  42.5  ; lunch
            Assets:Checking

        2026-01-01 Salary
            Assets:Checking  1000
            Income:Salary
        """
    )
    result = run_tally("print", path)
    assert result.returncode == 0
    assert result.stdout == (
        "2026-01-01 Salary\n"
        "    Assets:Checking  1000.00\n"
        "    Income:Salary  -1000.00\n"
        "\n"
        "2026-01-05 Grocery\n"
        "    Expenses:Food  42.50\n"
        "    Assets:Checking  -42.50\n"
    )
    reparsed = journal(result.stdout, name="printed.ledger")
    assert run_tally("check", reparsed).returncode == 0
