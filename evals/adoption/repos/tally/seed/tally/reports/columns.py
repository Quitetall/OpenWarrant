"""Shared column widths and cell formatting for text reports."""

from __future__ import annotations

from decimal import Decimal

from tally.amounts import format_amount

AMOUNT_WIDTH = 14
PAYEE_WIDTH = 20
ACCOUNT_WIDTH = 24


def amount_cell(value: Decimal) -> str:
    """An amount right-aligned in the standard amount column."""
    return f"{format_amount(value):>{AMOUNT_WIDTH}}"


def text_cell(text: str, width: int) -> str:
    """Text left-aligned in a column of ``width``, truncated to fit."""
    return f"{text:<{width}.{width}}"
