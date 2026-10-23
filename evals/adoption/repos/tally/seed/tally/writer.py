"""Writing transactions back out in journal syntax.

Output written here is itself a valid journal: every posting carries its
amount (elided amounts are filled in) and amounts have two decimal places.
"""

from __future__ import annotations

from tally.amounts import format_amount
from tally.model import Posting, Transaction

POSTING_INDENT = "    "


def render_posting(posting: Posting) -> str:
    return f"{POSTING_INDENT}{posting.account}  {format_amount(posting.amount)}"


def render_transaction(txn: Transaction) -> str:
    header = f"{txn.date.isoformat()} {txn.payee}".rstrip()
    return "\n".join([header, *(render_posting(p) for p in txn.postings)])


def render_transactions(transactions: list[Transaction]) -> str:
    """Transactions separated by one blank line."""
    return "\n\n".join(render_transaction(t) for t in transactions)
