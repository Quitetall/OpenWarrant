"""The accounts report: every account name the journal uses."""

from __future__ import annotations

from tally.model import Journal


def compute_accounts(journal: Journal) -> list[str]:
    """Distinct account names with at least one posting, sorted."""
    return sorted({p.account for t in journal.transactions for p in t.postings})


def render_accounts(names: list[str]) -> str:
    return "\n".join(names)
