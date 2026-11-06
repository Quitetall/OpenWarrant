from tally.parser import parse_text
from tally.reports.balance import compute_balances, render_balance
from tally.reports.register import compute_register

JOURNAL = """\
2026-01-05 Grocery
    Expenses:Food:Groceries  42.50
    Assets:Checking

2026-01-01 Salary
    Assets:Checking  1000
    Income:Salary

2026-01-09 Cafe
    Expenses:Food:Dining  7.25
    Assets:Checking
"""


def load():
    journal, problems = parse_text(JOURNAL, "j.ledger")
    assert problems == []
    return journal


def test_balances_sorted_by_account():
    rows = compute_balances(load())
    assert [(r.account, str(r.amount)) for r in rows] == [
        ("Assets:Checking", "950.25"),
        ("Expenses:Food:Dining", "7.25"),
        ("Expenses:Food:Groceries", "42.50"),
        ("Income:Salary", "-1000"),
    ]


def test_balance_filter_includes_descendants():
    rows = compute_balances(load(), account="Expenses:Food")
    assert [r.account for r in rows] == ["Expenses:Food:Dining", "Expenses:Food:Groceries"]
    assert render_balance(rows).splitlines()[-1] == "         49.75"


def test_register_is_in_date_order_with_running_total():
    rows = compute_register(load(), account="Assets:Checking")
    assert [(r.payee, str(r.running_total)) for r in rows] == [
        ("Salary", "1000"),
        ("Grocery", "957.50"),
        ("Cafe", "950.25"),
    ]
