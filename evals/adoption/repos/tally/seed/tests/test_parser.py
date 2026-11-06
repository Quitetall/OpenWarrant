import datetime as dt
from decimal import Decimal

from tally.parser import parse_text


def parse(text):
    return parse_text(text, "j.ledger")


def test_transaction_with_elided_amount():
    journal, problems = parse(
        "2026-01-05 Corner Grocery\n"
        "    Expenses:Food  42.50\n"
        "    Assets:Checking\n"
    )
    assert problems == []
    (txn,) = journal.transactions
    assert txn.date == dt.date(2026, 1, 5)
    assert txn.payee == "Corner Grocery"
    assert [p.amount for p in txn.postings] == [Decimal("42.50"), Decimal("-42.50")]
    assert txn.postings[1].inferred


def test_comments_and_account_names_with_spaces():
    journal, problems = parse(
        "; opening comment\n"
        "# hash comment\n"
        "2026-01-05 Rent\n"
        "    ; a comment inside the transaction\n"
        "    Expenses:Home Rent\t1200   ; inline\n"
        "    Assets:Joint Checking  -1200\n"
    )
    assert problems == []
    accounts = [p.account for p in journal.transactions[0].postings]
    assert accounts == ["Expenses:Home Rent", "Assets:Joint Checking"]


def test_problems_carry_file_and_line():
    _, problems = parse(
        "2026-01-05 A\n"
        "    Expenses:Food  10\n"
        "    Assets:Cash  -9\n"
        "\n"
        "2026-02-30 B\n"
        "    Expenses:Food  1\n"
        "\n"
        "    Assets:Cash  1\n"
    )
    assert [str(p) for p in problems] == [
        "j.ledger:1: transaction does not balance (off by 1.00)",
        "j.ledger:5: invalid date '2026-02-30'",
        "j.ledger:8: posting outside a transaction",
    ]


def test_two_elided_amounts_rejected():
    _, problems = parse("2026-01-05 A\n  Expenses:Food\n  Assets:Cash\n")
    assert [p.message for p in problems] == ["only one posting may omit its amount"]


def test_single_posting_rejected():
    _, problems = parse("2026-01-05 A\n  Expenses:Food  5\n")
    assert [p.message for p in problems] == ["transaction needs at least two postings"]


def test_bad_account_and_amount():
    _, problems = parse("2026-01-05 A\n  Expenses::Food  5\n  Assets:Cash  five\n")
    assert [(p.line, p.message) for p in problems] == [
        (2, "invalid account name 'Expenses::Food'"),
        (3, "invalid amount 'five'"),
    ]


def test_unrecognised_line():
    _, problems = parse("hello world\n  Assets:Cash  1\n")
    assert [(p.line, p.message) for p in problems] == [(1, "unrecognised line 'hello world'")]
