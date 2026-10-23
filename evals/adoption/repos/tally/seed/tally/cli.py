"""Command-line interface: ``python3 -m tally <command> <journal> [options]``.

Exit status:
    0  success
    1  ``check`` found problems in the journal
    2  usage error, unreadable file, or a journal with problems
"""

from __future__ import annotations

import argparse
import sys
from typing import Callable

from tally import __version__
from tally.errors import JournalError, UsageError
from tally.filters import add_date_range_options, date_range_from_args, restrict
from tally.model import Journal
from tally.parser import load_journal, parse_file
from tally.reports.accounts import compute_accounts, render_accounts
from tally.reports.balance import compute_balances, render_balance
from tally.reports.register import compute_register, render_register
from tally.writer import render_transactions

EXIT_OK = 0
EXIT_PROBLEMS = 1
EXIT_USAGE = 2


def load_selected(args: argparse.Namespace) -> Journal:
    """Load the journal named on the command line, applying --from/--until."""
    dates = date_range_from_args(args)
    if dates.is_empty():
        raise UsageError("--from must not be after --until")
    return restrict(load_journal(args.journal), dates)


def cmd_balance(args: argparse.Namespace) -> int:
    journal = load_selected(args)
    rows = compute_balances(journal, account=args.account)
    print(render_balance(rows))
    return EXIT_OK


def cmd_register(args: argparse.Namespace) -> int:
    journal = load_selected(args)
    rows = compute_register(journal, account=args.account)
    if rows:
        print(render_register(rows))
    return EXIT_OK


def cmd_accounts(args: argparse.Namespace) -> int:
    names = compute_accounts(load_journal(args.journal))
    if names:
        print(render_accounts(names))
    return EXIT_OK


def cmd_print(args: argparse.Namespace) -> int:
    journal = load_selected(args)
    transactions = journal.sorted_transactions()
    if transactions:
        print(render_transactions(transactions))
    return EXIT_OK


def cmd_check(args: argparse.Namespace) -> int:
    journal, problems = parse_file(args.journal)
    if problems:
        for problem in sorted(problems, key=lambda p: p.line):
            print(problem)
        print(f"{len(problems)} problem(s) found")
        return EXIT_PROBLEMS
    print(
        f"ok: {len(journal.transactions)} transactions, "
        f"{journal.posting_count()} postings"
    )
    return EXIT_OK


def _add_balance(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser("balance", help="net amount in each account")
    p.add_argument("journal", help="journal file")
    p.add_argument("account", nargs="?", help="limit to this account and its descendants")
    add_date_range_options(p)
    p.set_defaults(func=cmd_balance)


def _add_register(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser("register", help="postings in date order with a running total")
    p.add_argument("journal", help="journal file")
    p.add_argument("account", nargs="?", help="limit to this account and its descendants")
    add_date_range_options(p)
    p.set_defaults(func=cmd_register)


def _add_accounts(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser("accounts", help="list the accounts the journal uses")
    p.add_argument("journal", help="journal file")
    p.set_defaults(func=cmd_accounts)


def _add_print(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser("print", help="print transactions in date order, amounts filled in")
    p.add_argument("journal", help="journal file")
    add_date_range_options(p)
    p.set_defaults(func=cmd_print)


def _add_check(sub: argparse._SubParsersAction) -> None:
    p = sub.add_parser("check", help="validate a journal and report every problem")
    p.add_argument("journal", help="journal file")
    p.set_defaults(func=cmd_check)


# Each entry adds one subcommand. New commands register here.
COMMANDS: list[Callable[[argparse._SubParsersAction], None]] = [
    _add_balance,
    _add_register,
    _add_accounts,
    _add_print,
    _add_check,
]


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="tally", description="A plain-text double-entry ledger."
    )
    parser.add_argument("--version", action="version", version=f"tally {__version__}")
    sub = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")
    for add in COMMANDS:
        add(sub)
    return parser


def error(message: str) -> None:
    print(f"tally: error: {message}", file=sys.stderr)


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return args.func(args)
    except JournalError as exc:
        for problem in exc.problems:
            error(str(problem))
        return EXIT_USAGE
    except UsageError as exc:
        error(str(exc))
        return EXIT_USAGE
    except OSError as exc:
        error(f"{exc.filename}: {exc.strerror}")
        return EXIT_USAGE
