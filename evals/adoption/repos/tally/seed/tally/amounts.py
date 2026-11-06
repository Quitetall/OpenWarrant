"""Parsing and display of decimal amounts.

Amounts are kept as :class:`decimal.Decimal` at the precision they were
written with. Rounding happens only for display, to two places.
"""

from __future__ import annotations

import re
from decimal import ROUND_HALF_EVEN, Decimal

CENT = Decimal("0.01")
ZERO = Decimal("0")

_NUMBER_RE = re.compile(r"^[+-]?(\d+(\.\d*)?|\.\d+)$")


class AmountError(ValueError):
    """Text that is not a valid amount."""


def parse_amount(text: str) -> Decimal:
    """Parse a plain decimal number such as ``42``, ``-3.50`` or ``.25``."""
    text = text.strip()
    if not _NUMBER_RE.match(text):
        raise AmountError(f"invalid amount '{text}'")
    return Decimal(text)


def round_amount(value: Decimal) -> Decimal:
    """Round to two places, half to even."""
    return value.quantize(CENT, rounding=ROUND_HALF_EVEN)


def format_amount(value: Decimal) -> str:
    """Format an amount to exactly two decimal places (never ``-0.00``)."""
    rounded = round_amount(value)
    if rounded == ZERO:
        rounded = abs(rounded)
    return f"{rounded:.2f}"


def total(values) -> Decimal:
    """Sum an iterable of amounts, starting from an exact zero."""
    return sum(values, ZERO)
