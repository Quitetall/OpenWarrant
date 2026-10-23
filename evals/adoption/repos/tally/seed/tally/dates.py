"""ISO dates as they appear in journals and on the command line."""

from __future__ import annotations

import datetime as dt
import re

_DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


class DateError(ValueError):
    """Text that is not a valid YYYY-MM-DD calendar date."""


def parse_date(text: str) -> dt.date:
    """Parse ``YYYY-MM-DD``, rejecting impossible dates such as 2026-02-30."""
    if not _DATE_RE.match(text):
        raise DateError(f"invalid date '{text}'")
    try:
        return dt.date.fromisoformat(text)
    except ValueError:
        raise DateError(f"invalid date '{text}'") from None
