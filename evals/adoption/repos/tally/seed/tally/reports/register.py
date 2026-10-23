"""The register report: postings in date order with a running total."""

from __future__ import annotations

import datetime as dt
from dataclasses import dataclass
from decimal import Decimal

from tally.accounts import account_matches
from tally.amounts import ZERO
from tally.model import Journal
from tally.reports.columns import ACCOUNT_WIDTH, PAYEE_WIDTH, amount_cell, text_cell


@dataclass(frozen=True)
class RegisterRow:
    date: dt.date
    payee: str
    account: str
    amount: Decimal
    running_total: Decimal


def compute_register(journal: Journal, account: str | None = None) -> list[RegisterRow]:
    """One row per posting, in date order, with the running total so far.

    With ``account``, only postings to that account and its descendants are
    listed, and the running total covers only those postings.
    """
    rows: list[RegisterRow] = []
    running = ZERO
    for txn in journal.sorted_transactions():
        for posting in txn.postings:
            if account is not None and not account_matches(posting.account, account):
                continue
            running += posting.amount
            rows.append(RegisterRow(txn.date, txn.payee, posting.account, posting.amount, running))
    return rows


def render_register(rows: list[RegisterRow]) -> str:
    return "\n".join(
        " ".join(
            [
                row.date.isoformat(),
                text_cell(row.payee, PAYEE_WIDTH),
                text_cell(row.account, ACCOUNT_WIDTH),
                amount_cell(row.amount),
                amount_cell(row.running_total),
            ]
        )
        for row in rows
    )
