#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Derived views are written on main only (owner decision, 2026-11-07).

A pull request edits masters; after each push to main the Views workflow
compiles and commits the views. The paths are conformance/views.toml, the one
list. Every command here reads it, and nothing else decides what a view is.

  views.py pr --base REF          refuse a change that adds or edits a view
                                  (deleting one is allowed), or that touches a
                                  resolved Warrant's projection
  views.py changed                after `war compile` in this tree: print each
                                  changed view; refuse (exit 1) when anything
                                  that is not a view changed
  views.py fresh --war BIN [--commit]
                                  compile here, then `changed`; --commit makes a
                                  local commit of the views (disposable checkouts
                                  only: CI and plant clones, never pushed)
  views.py verify --war BIN       compile a scratch worktree of HEAD and run
                                  `war check --generated` there: the masters'
                                  views are checked fresh, never read stale
  views.py only-views --before A --after B
                                  exit 0 when A..B changed views and nothing else
  views.py drop --base REF        put this branch's views back as REF has them
  views.py number --commit SHA    append each id not yet numbered to the
                                  append-only index (the Views workflow only)

Standard library only (tomllib, Python 3.11+): CI's Python has no PyYAML.
"""
from __future__ import annotations

import argparse
import fnmatch
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIST = "conformance/views.toml"


def git(*args: str, cwd: Path | None = None, check: bool = True) -> str:
    out = subprocess.run(
        ["git", "--no-pager", *args],
        cwd=cwd or Path.cwd(),
        capture_output=True,
        text=True,
        timeout=600,
    )
    if check and out.returncode != 0:
        sys.exit(f"views: git {' '.join(args)} failed: {out.stderr.strip()}")
    return out.stdout


def load(_top: Path) -> dict:
    # The list beside this script: the one the running checks were written against.
    path = ROOT / LIST
    try:
        spec = tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as error:
        sys.exit(f"views: cannot read {LIST}: {error}")
    if spec.get("schema") != "ow.views/v1" or not spec.get("derived"):
        sys.exit(f"views: {LIST} is not an ow.views/v1 list with `derived` globs")
    return spec


def match(pattern: str, path: str) -> bool:
    """`*` is one path segment, `**` any number of them (zero included)."""

    def walk(p: list[str], s: list[str]) -> bool:
        if not p:
            return not s
        if p[0] == "**":
            return any(walk(p[1:], s[i:]) for i in range(len(s) + 1))
        return bool(s) and fnmatch.fnmatchcase(s[0], p[0]) and walk(p[1:], s[1:])

    return walk(pattern.split("/"), path.split("/"))


class Views:
    """What the list says about one path, against one tree."""

    def __init__(self, spec: dict):
        self.derived = list(spec["derived"])
        self.marker = spec.get("record_marker", "resolution.toml")
        self.record_glob = spec.get("record_glob", "docs/warrants/*/generated/**")

    def warrant_dir(self, path: str) -> str | None:
        if not match(self.record_glob, path):
            return None
        return path.split("/generated/", 1)[0]

    def is_listed(self, path: str) -> bool:
        return any(match(g, path) for g in self.derived)

    def kind(self, path: str, has_marker) -> str:
        """`record`, `derived`, or `master`. has_marker(dir) reads one tree."""
        w = self.warrant_dir(path)
        if w is not None and has_marker(w):
            return "record"
        return "derived" if self.is_listed(path) else "master"


def marker_in_commit(top: Path, commit: str, marker: str):
    def has(w: str) -> bool:
        out = subprocess.run(
            ["git", "cat-file", "-e", f"{commit}:{w}/{marker}"],
            cwd=top,
            capture_output=True,
            timeout=60,
        )
        return out.returncode == 0

    return has


def marker_on_disk(top: Path, marker: str):
    return lambda w: (top / w / marker).is_file()


def diff_paths(top: Path, a: str, b: str) -> list[str]:
    out = git("diff", "--name-only", "--no-renames", "-z", a, b, cwd=top)
    return [p for p in out.split("\0") if p]


def tree_changes(top: Path) -> list[str]:
    out = git("status", "--porcelain=v1", "-z", "--untracked-files=all", "--no-renames", cwd=top)
    return [entry[3:] for entry in out.split("\0") if len(entry) > 3]


def cmd_pr(args) -> int:
    top = Path(git("rev-parse", "--show-toplevel").strip())
    views = Views(load(top))
    base = git("merge-base", args.base, "HEAD", cwd=top).strip()
    at_base = marker_in_commit(top, base, views.marker)
    at_head = marker_in_commit(top, "HEAD", views.marker)
    refused = []
    status = git("diff", "--name-status", "--no-renames", "-z", base, "HEAD", cwd=top).split("\0")
    deleted = {status[i + 1] for i in range(0, len(status) - 1, 2) if status[i] == "D"}
    for path in diff_paths(top, base, "HEAD"):
        if views.kind(path, at_base) == "record":
            refused.append(
                f"views.record: {path}: a resolved Warrant's projection is part of its record; "
                "no change may regenerate, edit or drop it"
            )
        elif views.kind(path, at_head) == "record":
            # Resolved in this change: its projection becomes the record now,
            # and `verify` holds it to a fresh compilation.
            continue
        elif views.kind(path, at_base) == "derived" and path not in deleted:
            # A deletion is allowed: the views of a master this change removes
            # go with it, and main's compile restores any whose master remains.
            refused.append(
                f"views.pr: {path}: a derived view ({LIST}); pull requests carry masters "
                "only, and main's Views workflow writes views"
            )
    for line in refused:
        print(line)
    if refused:
        print(
            f"views: {len(refused)} path(s) refused; "
            f"`python3 conformance/views.py drop --base {args.base}` puts the views back"
        )
        return 1
    print(f"views.pr: pass (no view changed since {base[:12]})")
    return 0


def classify_tree(top: Path) -> tuple[list[str], list[str]]:
    views = Views(load(top))
    on_disk = marker_on_disk(top, views.marker)
    derived, other = [], []
    for path in tree_changes(top):
        (derived if views.kind(path, on_disk) == "derived" else other).append(path)
    return derived, other


def cmd_changed(_args) -> int:
    top = Path(git("rev-parse", "--show-toplevel").strip())
    derived, other = classify_tree(top)
    for path in derived:
        print(path)
    for path in other:
        print(
            f"views.unlisted: {path} changed, and it is not a derived view "
            f"(a master, or a resolved Warrant's projection); nothing here stages it",
            file=sys.stderr,
        )
    return 1 if other else 0


def compile_here(war: str, top: Path) -> None:
    out = subprocess.run(
        [war, "--root", str(top), "compile"], capture_output=True, text=True, timeout=1200
    )
    if out.returncode != 0:
        sys.stderr.write(out.stdout[-4000:] + out.stderr[-4000:])
        sys.exit(f"views: war compile failed (exit {out.returncode})")


def cmd_fresh(args) -> int:
    top = Path(git("rev-parse", "--show-toplevel").strip())
    war = str(Path(args.war).resolve())
    if tree_changes(top):
        sys.exit("views: fresh needs a clean tree; commit or stash first")
    compile_here(war, top)
    derived, other = classify_tree(top)
    for path in other:
        print(f"views.unlisted: compile changed {path}, which is not a derived view", file=sys.stderr)
    if other:
        return 1
    print(f"views: {len(derived)} view(s) recompiled")
    if args.commit and derived:
        git("add", "-A", "--", *derived, cwd=top)
        git(
            "-c", "user.name=views-local", "-c", "user.email=views-local@invalid",
            "-c", "commit.gpgsign=false",
            "commit", "-q", "--no-verify", "-m", "views: fresh for this run (local, never pushed)",
            cwd=top,
        )
    return 0


def cmd_verify(args) -> int:
    top = Path(git("rev-parse", "--show-toplevel").strip())
    war = str(Path(args.war).resolve())
    tmp_root = os.environ.get("TMPDIR") or tempfile.gettempdir()
    scratch = Path(tempfile.mkdtemp(prefix="ow-views-verify.", dir=tmp_root)).resolve()
    tree = scratch / "tree"
    try:
        git("worktree", "add", "-q", "--detach", str(tree), "HEAD", cwd=top)
        print(f"views: compiling HEAD ({git('rev-parse', '--short', 'HEAD', cwd=top).strip()}) in a scratch worktree")
        compile_here(war, tree)
        derived, other = classify_tree(tree)
        print(f"views: {len(derived)} committed view(s) differ from the masters' compilation (expected off main)")
        for path in other:
            print(
                f"views.unlisted: compile changed {path}, which is not a derived view "
                "(a resolved Warrant's projection is part of its record)"
            )
        check = subprocess.run([war, "--root", str(tree), "check", "--generated"], timeout=1200)
        if check.returncode != 0:
            print(f"views: war check --generated over the fresh views: exit {check.returncode}")
        return 1 if other or check.returncode != 0 else 0
    finally:
        git("worktree", "remove", "--force", str(tree), cwd=top, check=False)
        if scratch.name.startswith("ow-views-verify.") and scratch.parent == Path(tmp_root).resolve():
            shutil.rmtree(scratch, ignore_errors=True)


def cmd_only_views(args) -> int:
    top = Path(git("rev-parse", "--show-toplevel").strip())
    views = Views(load(top))
    if set(args.before) == {"0"}:
        print("views: no previous commit; not a views-only push")
        return 1
    paths = diff_paths(top, args.before, args.after)
    at_after = marker_in_commit(top, args.after, views.marker)
    rest = [p for p in paths if views.kind(p, at_after) != "derived"]
    if paths and not rest:
        print(f"views: {len(paths)} path(s) changed, every one a view")
        return 0
    print(f"views: {len(rest)} non-view path(s) changed")
    return 1


def cmd_drop(args) -> int:
    top = Path(git("rev-parse", "--show-toplevel").strip())
    views = Views(load(top))
    base = git("merge-base", args.base, "HEAD", cwd=top).strip()
    at_base = marker_in_commit(top, base, views.marker)
    changed = set(diff_paths(top, base, "HEAD")) | set(tree_changes(top))
    mine = sorted(p for p in changed if views.kind(p, at_base) == "derived")
    in_base = set(git("ls-tree", "-r", "--name-only", "-z", base, "--", *mine, cwd=top).split("\0")) if mine else set()
    keep = [p for p in mine if p in in_base]
    gone = [p for p in mine if p not in in_base]
    if keep:
        git("restore", f"--source={base}", "--staged", "--worktree", "--", *keep, cwd=top)
    tracked = set(git("ls-files", "-z", "--", *gone, cwd=top).split("\0")) if gone else set()
    if tracked - {""}:
        git("rm", "-q", "-f", "--cached", "--", *sorted(tracked - {""}), cwd=top)
    for path in gone:
        (top / path).unlink(missing_ok=True)
    print(f"views: {len(keep)} view(s) restored from {base[:12]}, {len(gone)} removed; commit the result")
    return 0


HEADER = "id\tnumber\tfirst_seen"


def read_index(text: str) -> list[tuple[str, int, str]]:
    lines = text.splitlines()
    if not lines or lines[0] != HEADER:
        sys.exit(f"views.numbers: the index does not start with the header {HEADER!r}")
    rows, seen = [], set()
    for n, line in enumerate(lines[1:], start=2):
        parts = line.split("\t")
        if len(parts) != 3 or not parts[1].isdigit():
            sys.exit(f"views.numbers: line {n} is not `id<TAB>number<TAB>commit`: {line!r}")
        number = int(parts[1])
        if parts[0] in seen or number != len(rows) + 1:
            sys.exit(f"views.numbers: line {n}: {parts[0]} {number}: an id twice, or a number out of sequence")
        seen.add(parts[0])
        rows.append((parts[0], number, parts[2]))
    return rows


def cmd_number(args) -> int:
    top = Path(git("rev-parse", "--show-toplevel").strip())
    spec = load(top).get("numbers") or {}
    index, sources = spec.get("index"), spec.get("sources") or []
    if not index:
        print("views.numbers: no [numbers] index configured")
        return 0
    path = top / index
    before = path.read_text() if path.is_file() else HEADER + "\n"
    committed = subprocess.run(
        ["git", "show", f"HEAD:{index}"], cwd=top, capture_output=True, text=True, timeout=60
    )
    if committed.returncode == 0 and not before.startswith(committed.stdout):
        sys.exit(
            f"views.numbers: {index} differs from the committed index other than by appending; "
            "a number is never changed or reused"
        )
    rows = read_index(before)
    known = {row[0] for row in rows}
    tracked = [p for p in git("ls-files", "-z", cwd=top).split("\0") if p]
    ids = set()
    for p in tracked:
        for glob in sources:
            depth = len(glob.split("/"))
            parts = p.split("/")
            if len(parts) >= depth and match(glob, "/".join(parts[:depth])):
                ids.add(parts[depth - 1].split(".", 1)[0])
    new = sorted(i for i in ids if i not in known)
    if not new:
        print(f"views.numbers: {len(rows)} id(s) numbered, none new")
        return 0
    after = before if before.endswith("\n") else before + "\n"
    for offset, ident in enumerate(new, start=1):
        if "\t" in ident or "\n" in ident:
            sys.exit(f"views.numbers: id {ident!r} cannot be written to a TSV row")
        after += f"{ident}\t{len(rows) + offset}\t{args.commit}\n"
    # Append-only, by construction and checked: the old index is a prefix.
    assert after.startswith(before.rstrip("\n"))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(after)
    print(f"views.numbers: {len(new)} new id(s) numbered {len(rows) + 1}..{len(rows) + len(new)}")
    return 0


def main() -> int:
    sys.stdout.reconfigure(line_buffering=True)
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("pr")
    p.add_argument("--base", required=True)
    sub.add_parser("changed")
    p = sub.add_parser("fresh")
    p.add_argument("--war", required=True)
    p.add_argument("--commit", action="store_true")
    p = sub.add_parser("verify")
    p.add_argument("--war", required=True)
    p = sub.add_parser("only-views")
    p.add_argument("--before", required=True)
    p.add_argument("--after", required=True)
    p = sub.add_parser("drop")
    p.add_argument("--base", required=True)
    p = sub.add_parser("number")
    p.add_argument("--commit", required=True)
    args = parser.parse_args()
    return {
        "pr": cmd_pr,
        "changed": cmd_changed,
        "fresh": cmd_fresh,
        "verify": cmd_verify,
        "only-views": cmd_only_views,
        "drop": cmd_drop,
        "number": cmd_number,
    }[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
