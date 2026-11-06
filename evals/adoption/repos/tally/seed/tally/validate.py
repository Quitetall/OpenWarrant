"""Rules every transaction must satisfy once it has been read."""

from __future__ import annotations

from tally.amounts import ZERO, format_amount
from tally.errors import Problem
from tally.model import Transaction


def finalize(txn: Transaction) -> list[Problem]:
    """Fill in an elided amount and check that ``txn`` balances.

    Returns the problems found; an empty list means the transaction is
    complete and every posting has an amount.
    """
    problems: list[Problem] = []

    def problem(line: int, message: str) -> None:
        problems.append(Problem(txn.path, line, message))

    if len(txn.postings) < 2:
        problem(txn.line, "transaction needs at least two postings")
        return problems

    elided = txn.elided_postings()
    if len(elided) > 1:
        problem(elided[1].line, "only one posting may omit its amount")
        return problems

    residual = txn.residual()
    if elided:
        elided[0].amount = -residual if residual != ZERO else ZERO
        elided[0].inferred = True
    elif residual != ZERO:
        problem(txn.line, f"transaction does not balance (off by {format_amount(residual)})")
    return problems
