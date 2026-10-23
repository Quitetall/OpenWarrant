"""The balance report: the net amount in each account."""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
from decimal import Decimal

from tally.accounts import account_matches
from tally.amounts import ZERO, total
from tally.model import Journal
from tally.reports.columns import AMOUNT_WIDTH, amount_cell


@dataclass(frozen=True)
class BalanceRow:
    account: str
    amount: Decimal


def compute_balances(journal: Journal, account: str | None = None) -> list[BalanceRow]:
    """Net amount per account that has postings, sorted by account name.

    With ``account``, only that account and its descendants are included.
    """
    sums: dict[str, Decimal] = defaultdict(lambda: ZERO)
    for txn in journal.transactions:
        for posting in txn.postings:
            if account is not None and not account_matches(posting.account, account):
                continue
            sums[posting.account] += posting.amount
    return [BalanceRow(name, sums[name]) for name in sorted(sums)]


def render_balance(rows: list[BalanceRow]) -> str:
    lines = [f"{amount_cell(row.amount)}  {row.account}" for row in rows]
    lines.append("-" * AMOUNT_WIDTH)
    lines.append(amount_cell(total(row.amount for row in rows)))
    return "\n".join(lines)
