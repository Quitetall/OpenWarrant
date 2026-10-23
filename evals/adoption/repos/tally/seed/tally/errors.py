"""Problems found in a journal, and the exceptions that carry them."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Problem:
    """One thing wrong with a journal, located by file and line."""

    path: str
    line: int
    message: str

    def __str__(self) -> str:
        return f"{self.path}:{self.line}: {self.message}"


class JournalError(Exception):
    """A journal could not be loaded because it has problems."""

    def __init__(self, problems: list[Problem]):
        self.problems = list(problems)
        super().__init__("; ".join(str(p) for p in self.problems))


class UsageError(Exception):
    """A command was invoked with arguments it cannot act on."""
