#!/usr/bin/env python3
"""Agent-facing text lint (M9, decision 19): no unscoped prohibition.

An agent that reads "you may not ..." in a repository's instructions stops
doing ordinary work. What the tool refuses belongs in a section scoped to
the Warrant it applies to, said as what the tool does.

Usage: lint.py [--first-line] LABEL=FILE ...

Each FILE is read as Markdown. A heading whose text names a Warrant and its
sign-off ("When a Warrant's type requires sign-off", "Warrant sign-off: the
loop") opens a scoped section that lasts until the next heading of the same
or a higher level; inside it the words below are allowed, because there they
describe what the tool refuses for that Warrant. Everywhere else these are
refused, case-insensitively, as whole words:

    may not, must not, never, forbidden, not permitted, stop

--first-line lints only the first non-empty line of each file: the line an
empty-state message leads with.

Prints `prohibition: LABEL:LINE: "<word>": <text>` per finding and exits 1
when there is any; prints `agent-text: N file(s), M line(s), clean` and exits
0 otherwise. A file that cannot be read exits 2: UNKNOWN, not clean.
"""
import re
import sys

WORDS = re.compile(r"\b(may not|must not|never|forbidden|not permitted|stop)\b", re.I)
HEADING = re.compile(r"^(#{1,6})\s+(.*)$")


def scoped(title: str) -> bool:
    return bool(re.search(r"warrant", title, re.I) and re.search(r"sign[- ]off", title, re.I))


def lint(label: str, text: str, first_line: bool) -> tuple[list[str], int]:
    findings = []
    lines = text.splitlines()
    if first_line:
        lines = [next((l for l in lines if l.strip()), "")]
    scope_level = None  # heading level of the open scoped section
    for n, line in enumerate(lines, 1):
        h = HEADING.match(line)
        if h:
            level = len(h.group(1))
            if scope_level is not None and level <= scope_level:
                scope_level = None
            if scope_level is None and scoped(h.group(2)):
                scope_level = level
                continue
        if scope_level is not None:
            continue
        m = WORDS.search(line)
        if m:
            findings.append(f'prohibition: {label}:{n}: "{m.group(1)}": {line.strip()}')
    return findings, len(lines)


def main(argv: list[str]) -> int:
    first_line = False
    if argv and argv[0] == "--first-line":
        first_line, argv = True, argv[1:]
    if not argv:
        print("usage: lint.py [--first-line] LABEL=FILE ...", file=sys.stderr)
        return 2
    findings, total = [], 0
    for arg in argv:
        label, _, path = arg.partition("=")
        if not path:
            label, path = arg, arg
        try:
            with open(path, encoding="utf-8") as f:
                text = f.read()
        except OSError as e:
            print(f"agent-text: {label}: could not be read ({e}); UNKNOWN", file=sys.stderr)
            return 2
        got, n = lint(label, text, first_line)
        findings += got
        total += n
    for f in findings:
        print(f)
    if findings:
        return 1
    print(f"agent-text: {len(argv)} file(s), {total} line(s), clean")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
