"""The in-memory journal: transactions and their postings."""

from __future__ import annotations

import datetime as dt
from dataclasses import dataclass, field
from decimal import Decimal

from tally.amounts import total


@dataclass
class Posting:
    """One leg of a transaction: an amount moved into or out of an account.

    ``amount`` is None while the posting's amount is elided in the source;
    it is filled in when the transaction is finalized.
    """

    account: str
    amount: Decimal | None
    line: int
    inferred: bool = False


@dataclass
class Transaction:
    """A dated, balanced set of postings."""

    date: dt.date
    payee: str
    path: str
    line: int
    postings: list[Posting] = field(default_factory=list)

    def explicit_postings(self) -> list[Posting]:
        return [p for p in self.postings if p.amount is not None]

    def elided_postings(self) -> list[Posting]:
        return [p for p in self.postings if p.amount is None]

    def residual(self) -> Decimal:
        """Sum of the explicit amounts; zero when the transaction balances."""
        return total(p.amount for p in self.explicit_postings())


@dataclass
class Journal:
    """Everything read from one journal file."""

    path: str
    transactions: list[Transaction] = field(default_factory=list)

    def sorted_transactions(self) -> list[Transaction]:
        """Transactions by date; same-day transactions keep file order."""
        return sorted(self.transactions, key=lambda t: t.date)

    def posting_count(self) -> int:
        return sum(len(t.postings) for t in self.transactions)
