"""Line-oriented journal parser.

The parser does not stop at the first problem. It records each one with its
file and line and carries on, so ``tally check`` can report all of them at
once. Commands that need a sound journal use :func:`load_journal`, which
raises :class:`~tally.errors.JournalError` if anything was found.

Syntax overview (see docs/FORMAT.md for the full description)::

    ; a comment
    2026-01-05 Corner Grocery
        Expenses:Food        42.50   ; inline comment
        Assets:Checking
"""

from __future__ import annotations

import re
from pathlib import Path

from tally.accounts import account_problem
from tally.amounts import AmountError, parse_amount
from tally.dates import DateError, parse_date
from tally.errors import JournalError, Problem
from tally.model import Journal, Posting, Transaction
from tally.validate import finalize

COMMENT_CHARS = (";", "#")

# An account is separated from its amount by two or more spaces or a tab,
# so account names may contain single spaces.
_ACCOUNT_AMOUNT_SPLIT = re.compile(r" {2,}|\t")


class Parser:
    """Reads journal text into a :class:`Journal`, collecting problems."""

    def __init__(self, path: str):
        self.path = path
        self.journal = Journal(path)
        self.problems: list[Problem] = []
        self._current: Transaction | None = None
        # True while skipping the indented lines of an entry whose first
        # line was already reported, so one mistake is reported once.
        self._skipping = False
        # True once a line of the current transaction failed to parse; the
        # transaction is then dropped without balance checks.
        self._broken = False

    def parse(self, text: str) -> tuple[Journal, list[Problem]]:
        for lineno, raw in enumerate(text.splitlines(), start=1):
            self._read_line(lineno, raw)
        self._close_entry()
        return self.journal, self.problems

    # -- line dispatch ---------------------------------------------------

    def _read_line(self, lineno: int, raw: str) -> None:
        stripped = raw.strip()
        if not stripped:
            self._close_entry()
            return
        if stripped.startswith(COMMENT_CHARS):
            return
        if raw[0] in " \t":
            self._read_indented(lineno, stripped)
            return
        self._close_entry()
        self._read_directive(lineno, raw.rstrip())

    def _read_directive(self, lineno: int, line: str) -> None:
        """A line starting in column one opens a new entry."""
        if line[0].isdigit():
            self._read_transaction_header(lineno, line)
        else:
            self._problem(lineno, f"unrecognised line '{line}'")
            self._skipping = True

    def _read_indented(self, lineno: int, text: str) -> None:
        if self._skipping:
            return
        if self._current is None:
            self._problem(lineno, "posting outside a transaction")
            return
        posting = self._parse_posting(lineno, text)
        if posting is None:
            self._broken = True
        else:
            self._current.postings.append(posting)

    # -- entries ---------------------------------------------------------

    def _read_transaction_header(self, lineno: int, line: str) -> None:
        date_text, _, payee = line.partition(" ")
        try:
            date = parse_date(date_text)
        except DateError as exc:
            self._problem(lineno, str(exc))
            self._skipping = True
            return
        self._current = Transaction(date=date, payee=payee.strip(), path=self.path, line=lineno)

    def _parse_posting(self, lineno: int, text: str) -> Posting | None:
        text = _strip_comment(text)
        parts = _ACCOUNT_AMOUNT_SPLIT.split(text, maxsplit=1)
        account = parts[0].strip()
        problem = account_problem(account)
        if problem:
            self._problem(lineno, problem)
            return None
        amount_text = parts[1].strip() if len(parts) > 1 else ""
        if not amount_text:
            return Posting(account=account, amount=None, line=lineno)
        try:
            amount = parse_amount(amount_text)
        except AmountError as exc:
            self._problem(lineno, str(exc))
            return None
        return Posting(account=account, amount=amount, line=lineno)

    def _close_entry(self) -> None:
        if self._current is not None and not self._broken:
            problems = finalize(self._current)
            if problems:
                self.problems.extend(problems)
            else:
                self.journal.transactions.append(self._current)
        self._current = None
        self._skipping = False
        self._broken = False

    def _problem(self, lineno: int, message: str) -> None:
        self.problems.append(Problem(self.path, lineno, message))


def _strip_comment(text: str) -> str:
    for char in COMMENT_CHARS:
        index = text.find(char)
        if index != -1:
            text = text[:index]
    return text.rstrip()


def parse_text(text: str, path: str = "<string>") -> tuple[Journal, list[Problem]]:
    """Parse journal text; return the journal and every problem found."""
    return Parser(path).parse(text)


def parse_file(path: str) -> tuple[Journal, list[Problem]]:
    """Parse a journal file. ``path`` is reported exactly as given."""
    text = Path(path).read_text(encoding="utf-8")
    return parse_text(text, path)


def load_journal(path: str) -> Journal:
    """Parse a journal file, raising JournalError if it has any problem."""
    journal, problems = parse_file(path)
    if problems:
        raise JournalError(sorted(problems, key=lambda p: p.line))
    return journal
