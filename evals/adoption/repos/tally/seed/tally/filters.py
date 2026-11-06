"""Selecting which transactions a report looks at."""

from __future__ import annotations

import argparse
import datetime as dt
from dataclasses import dataclass

from tally.dates import DateError, parse_date
from tally.model import Journal, Transaction


@dataclass(frozen=True)
class DateRange:
    """An inclusive date range; either end may be open."""

    start: dt.date | None = None
    end: dt.date | None = None

    def contains(self, date: dt.date) -> bool:
        if self.start is not None and date < self.start:
            return False
        if self.end is not None and date > self.end:
            return False
        return True

    def is_empty(self) -> bool:
        return self.start is not None and self.end is not None and self.start > self.end


def restrict(journal: Journal, dates: DateRange) -> Journal:
    """A copy of ``journal`` holding only the transactions within ``dates``."""
    kept: list[Transaction] = [t for t in journal.transactions if dates.contains(t.date)]
    return Journal(path=journal.path, transactions=kept)


def date_argument(text: str) -> dt.date:
    """``type=`` callable for argparse options that take a date."""
    try:
        return parse_date(text)
    except DateError as exc:
        raise argparse.ArgumentTypeError(str(exc)) from None


def add_date_range_options(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--from", dest="date_from", type=date_argument, metavar="DATE",
        help="only transactions on or after DATE (YYYY-MM-DD)",
    )
    parser.add_argument(
        "--until", dest="date_until", type=date_argument, metavar="DATE",
        help="only transactions on or before DATE (YYYY-MM-DD)",
    )


def date_range_from_args(args: argparse.Namespace) -> DateRange:
    return DateRange(start=args.date_from, end=args.date_until)
