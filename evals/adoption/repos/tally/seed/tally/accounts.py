"""Account names.

An account name is a sequence of segments joined by ``:``, for example
``Expenses:Food:Groceries``. Segments may contain single spaces but not
leading or trailing whitespace, and none may be empty.
"""

from __future__ import annotations

SEPARATOR = ":"


def account_problem(name: str) -> str | None:
    """Return why ``name`` is not a valid account name, or None if it is."""
    segments = name.split(SEPARATOR)
    for segment in segments:
        if not segment or segment != segment.strip() or "  " in segment:
            return f"invalid account name '{name}'"
    return None


def segments(name: str) -> list[str]:
    """Split an account name into its segments."""
    return name.split(SEPARATOR)


def account_matches(name: str, pattern: str) -> bool:
    """True if ``name`` is ``pattern`` or one of its descendants.

    ``Expenses:Food`` matches ``Expenses:Food`` and ``Expenses:Food:Groceries``
    but not ``Expenses:Foodstuffs``.
    """
    return name == pattern or name.startswith(pattern + SEPARATOR)
