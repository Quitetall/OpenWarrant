from decimal import Decimal

import pytest

from tally.amounts import AmountError, format_amount, parse_amount


@pytest.mark.parametrize(
    "text, expected",
    [("42", "42"), ("-3.50", "-3.50"), ("+7.1", "7.1"), (".25", "0.25"), ("1.005", "1.005")],
)
def test_parse_amount(text, expected):
    assert parse_amount(text) == Decimal(expected)


@pytest.mark.parametrize("text", ["", "abc", "1,000", "1.2.3", "--4", "$5"])
def test_parse_amount_rejects(text):
    with pytest.raises(AmountError):
        parse_amount(text)


@pytest.mark.parametrize(
    "value, expected",
    [("42", "42.00"), ("-3.5", "-3.50"), ("1.005", "1.00"), ("1.015", "1.02"), ("-0.001", "0.00")],
)
def test_format_amount(value, expected):
    assert format_amount(Decimal(value)) == expected
