#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Which plant files run where. The one definition behind the shards and the PR tier.

    plant-select.py shard i/n
        The plant files of shard i of n, one per line. `plant.sh` calls this for
        OW_PLANT_SHARD. A greedy partition by `plant-weights.tsv`: every file is
        assigned to exactly one shard, whatever the weights say, and a stale or
        missing weight costs balance, never a plant.

    plant-select.py affected --base REV --shard i/n --cap SECONDS [--out FILE]
        The plants a change touches (`plant-map.toml`), for Fast CI. Everything
        it decides is printed: which rule each changed path met, which paths no
        rule names, what runs and what the cap left out.

    plant-select.py reports DIR --of n [--weights-out FILE]
        What the shards say they actually ran (OW_PLANT_REPORT), for the job
        that closes the checkpoint: every plant file exactly once, every shard
        accounted for, and the one total.

    plant-select.py check-map
        Every rule of the map still names something real.

What a selection may not do is shrink silently, so each doubt falls towards
running more:

    a changed path no rule names          -> every plant (and a visible warning)
    a rule whose plant pattern is dead    -> every plant (and a visible warning)
    a plant no rule names                 -> always selected
    the base cannot be read               -> every plant (and a visible warning)

Only the cap takes a selected plant away, and each one it takes is named as NOT
RUN. That is UNKNOWN, not PASS: the full gate still runs it (Law 15).

Standard library only: this runs on a bare runner, before any `pip install`.
"""

import argparse
import fnmatch
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLANTS_DIR = "conformance/plants.d"
MAP_FILE = "conformance/plant-map.toml"
WEIGHTS_FILE = "conformance/plant-weights.tsv"

# A plant with no measured time. Above the median on purpose: an unmeasured
# plant should not look cheap to the cap or to the balance.
DEFAULT_SECONDS = 30

# The order a cap reads a selection in: what the change touched first.
TIER_CHANGED = 0  # the plant file itself changed
TIER_MAPPED = 1  # a rule connects a changed path to it
TIER_UNMAPPED = 2  # no rule names this plant, so it always runs
TIER_WIDE = 3  # selected only because the answer fell to "every plant"
TIER_NAMES = {
    TIER_CHANGED: "changed",
    TIER_MAPPED: "mapped",
    TIER_UNMAPPED: "no-mapping",
    TIER_WIDE: "widened",
}


def die(message):
    print(f"plant-select: {message}", file=sys.stderr)
    sys.exit(2)


def plant_files():
    """The plant files, in the byte order `plant.sh` runs them in."""
    names = [p.name for p in (ROOT / PLANTS_DIR).glob("*.sh")]
    return sorted(names, key=os.fsencode)


def weights():
    """Seconds per plant file. Lines are `<seconds>\\t<file>`; # starts a comment."""
    out = {}
    path = ROOT / WEIGHTS_FILE
    if not path.exists():
        return out
    for number, line in enumerate(path.read_text().splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            seconds, name = line.split("\t")
            out[name] = int(seconds)
        except ValueError:
            die(f"{WEIGHTS_FILE}:{number}: wanted '<seconds><TAB><file>', got {line!r}")
    return out


def cost(name, table):
    return table.get(name, DEFAULT_SECONDS)


def partition(files, table, n):
    """Longest job first, each to the least-loaded shard; ties to the lowest index.

    Deterministic, so every shard of a matrix computes the same answer alone.
    Within a shard the files keep byte order, the order `plant.sh` documents.
    """
    shards = [[] for _ in range(n)]
    loads = [0] * n
    for name in sorted(files, key=lambda f: (-cost(f, table), os.fsencode(f))):
        least = loads.index(min(loads))
        shards[least].append(name)
        loads[least] += cost(name, table)
    return [sorted(s, key=os.fsencode) for s in shards], loads


def parse_shard(text):
    match = re.fullmatch(r"([0-9]+)/([0-9]+)", text or "")
    if not match:
        die(f"a shard is i/n with 1 <= i <= n, not {text!r}")
    i, n = int(match.group(1)), int(match.group(2))
    if not 1 <= i <= n:
        die(f"a shard is i/n with 1 <= i <= n, not {text!r}")
    return i, n


def glob_regex(pattern):
    """`**/` crosses directories, `**` at the end takes the rest, `*` stops at a slash."""
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out += r"(?:.*/)?"
            i += 3
        elif pattern.startswith("**", i):
            out += r".*"
            i += 2
        elif pattern[i] == "*":
            out += r"[^/]*"
            i += 1
        elif pattern[i] == "?":
            out += r"[^/]"
            i += 1
        else:
            out += re.escape(pattern[i])
            i += 1
    return re.compile(out + r"\Z")


def load_map():
    path = ROOT / MAP_FILE
    try:
        document = tomllib.loads(path.read_text())
    except (OSError, tomllib.TOMLDecodeError) as error:
        die(f"{MAP_FILE}: {error}")
    rules = document.get("rule")
    if not isinstance(rules, list) or not rules:
        die(f"{MAP_FILE}: no [[rule]]; a map that names nothing is a broken map")
    for rule in rules:
        for key in ("name", "paths", "plants"):
            if key not in rule:
                die(f"{MAP_FILE}: a rule has no {key!r}: {rule}")
        rule["regexes"] = [glob_regex(p) for p in rule["paths"]]
    return rules


def plants_of(rule, files):
    """(the files a rule names, the patterns of it that name nothing)."""
    named, dead = set(), []
    for pattern in rule["plants"]:
        hits = {f for f in files if fnmatch.fnmatchcase(f, pattern)}
        named |= hits
        if not hits:
            dead.append(pattern)
    return named, dead


def changed_paths(base):
    """The paths between `base` and HEAD, or None when that cannot be read."""
    try:
        result = subprocess.run(
            ["git", "-C", str(ROOT), "diff", "--name-only", "--no-renames", "-z", base, "HEAD", "--"],
            capture_output=True,
            timeout=120,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    return [p for p in result.stdout.decode("utf-8", "surrogateescape").split("\0") if p]


class Selection:
    """Tier per selected plant, and the lines that say how each came to be."""

    def __init__(self):
        self.tier = {}
        self.log = []
        self.warnings = []
        self.wide = False

    def add(self, name, tier):
        self.tier[name] = min(tier, self.tier.get(name, tier))

    def warn(self, message):
        self.warnings.append(message)
        self.wide = True


def select(changed, files, rules):
    sel = Selection()
    if changed is None:
        sel.warn("the changed files could not be read; every plant is selected")
        changed = []
    plant_prefix = PLANTS_DIR + "/"
    for path in changed:
        direct = path.startswith(plant_prefix) and path[len(plant_prefix):] in files
        if direct:
            sel.add(path[len(plant_prefix):], TIER_CHANGED)
            sel.log.append(f"  changed   {path}: the plant itself")
        # A plant file may also be a path of a rule: the plants that run it
        # inside themselves. One no rule names selects itself and nothing more.
        met = [r for r in rules if any(rx.match(path) for rx in r["regexes"])]
        if direct and not met:
            continue
        if not met:
            sel.warn(f"{path}: no rule in {MAP_FILE} names this path; every plant is selected")
            sel.log.append(f"  UNMAPPED  {path}")
            continue
        for rule in met:
            if rule["plants"] == ["*"]:
                sel.wide = True
                sel.log.append(f"  everything {path}: rule {rule['name']}")
                continue
            named, dead = plants_of(rule, files)
            for pattern in dead:
                sel.warn(f"rule {rule['name']}: {pattern!r} names no plant; every plant is selected")
            for name in named:
                sel.add(name, TIER_MAPPED)
            sel.log.append(f"  changed   {path}: rule {rule['name']} ({len(named)} plant(s))")
    # A plant no rule names has no way to be selected by a path, so it is
    # selected by every change: new plants run until somebody maps them.
    mapped = set()
    for rule in rules:
        if rule["plants"] != ["*"]:
            mapped |= plants_of(rule, files)[0]
    for name in files:
        if name not in mapped:
            sel.add(name, TIER_UNMAPPED)
    if sel.wide:
        for name in files:
            sel.add(name, TIER_WIDE)
    return sel


def apply_cap(tiers, table, total):
    """(run, not run). Tier order, cheapest first inside a tier, so a cap costs
    the fewest plants and takes what the change did not touch before what it did.
    A plant that does not fit is passed over, and cheaper ones still run."""
    run, left, spent = [], [], 0
    for name in sorted(tiers, key=lambda f: (tiers[f], cost(f, table), os.fsencode(f))):
        if spent + cost(name, table) <= total:
            run.append(name)
            spent += cost(name, table)
        else:
            left.append(name)
    return run, left, spent


def cmd_shard(args):
    i, n = parse_shard(args.shard)
    shards, _ = partition(plant_files(), weights(), n)
    for name in shards[i - 1]:
        print(name)


def cmd_affected(args):
    i, n = parse_shard(args.shard)
    files = plant_files()
    if not files:
        die(f"{PLANTS_DIR} holds no plants")
    table = weights()
    changed = changed_paths(args.base)
    sel = select(changed, files, load_map())
    run, left, spent = apply_cap(sel.tier, table, args.cap * n)
    shards, loads = partition(run, table, n)

    lines = []
    count = "unreadable" if changed is None else str(len(changed))
    lines.append(f"plant-select: {count} changed path(s) against {args.base}")
    lines += sel.log
    by_tier = {}
    for name in run:
        by_tier.setdefault(sel.tier[name], []).append(name)
    summary = ", ".join(f"{len(v)} {TIER_NAMES[t]}" for t, v in sorted(by_tier.items()))
    lines.append(
        f"plant-select: {len(run)} of {len(files)} plant file(s) selected"
        f" ({summary or 'none'}); about {spent} s against a cap of {args.cap * n} s"
    )
    lines.append(f"plant-select: this leg is {i}/{n}: {len(shards[i - 1])} file(s), about {loads[i - 1]} s")
    for name in shards[i - 1]:
        lines.append(f"  RUN       {name} ({TIER_NAMES[sel.tier[name]]})")
    for name in left:
        lines.append(f"  NOT RUN   {name} ({TIER_NAMES[sel.tier[name]]}, {cost(name, table)} s): time cap; unknown here, the full gate runs it")
    print("\n".join(lines), flush=True)

    # Annotations reach the pull request's page, not only the job log.
    in_actions = bool(os.environ.get("GITHUB_ACTIONS"))
    for message in sel.warnings:
        print(f"::warning::{message}" if in_actions else f"WARNING: {message}")
    if left:
        message = f"{len(left)} selected plant file(s) over the time cap were not run"
        print(f"::warning::{message}" if in_actions else f"WARNING: {message}")
    summary_file = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_file and i == 1:
        with open(summary_file, "a") as out:
            out.write("### Plants this change touches\n\n```\n" + "\n".join(lines) + "\n```\n")
    if args.out:
        Path(args.out).write_text("".join(name + "\n" for name in shards[i - 1]))


def read_report(path):
    """One shard's report, or the reason it is not one."""
    report = {"shard": None, "fixture": None, "files": [], "total": None}
    for line in path.read_text().splitlines():
        words = line.split()
        if len(words) == 2 and words[0] == "shard":
            report["shard"] = words[1]
        elif len(words) == 3 and words[0] == "fixture":
            report["fixture"] = (int(words[1]), int(words[2]))
        elif len(words) == 3 and words[0] == "file":
            report["files"].append((words[1], int(words[2])))
        elif len(words) == 3 and words[0] == "total":
            report["total"] = (int(words[1]), int(words[2]))
        else:
            return None, f"{path.name}: unreadable line {line!r}"
    # `total` is written last: a shard that died part way has none.
    if None in (report["shard"], report["fixture"], report["total"]):
        return None, f"{path.name}: incomplete report"
    return report, None


def cmd_reports(args):
    problems, reports = [], {}
    for path in sorted(Path(args.dir).rglob("*.report")):
        report, why = read_report(path)
        if why:
            problems.append(why)
        elif report["shard"] in reports:
            problems.append(f"shard {report['shard']} reported twice")
        else:
            reports[report["shard"]] = report
    wanted = {f"{i}/{args.of}" for i in range(1, args.of + 1)}
    for label in sorted(wanted - reports.keys()):
        problems.append(f"shard {label} sent no report (it did not finish, or did not run)")
    for label in sorted(reports.keys() - wanted):
        problems.append(f"shard {label} is not one of {args.of}: the matrix and this check disagree")

    seen = {}
    seconds = {}
    for label, report in sorted(reports.items()):
        for name, secs in report["files"]:
            seen.setdefault(name, []).append(label)
            seconds[name] = secs
    files = plant_files()
    for name in files:
        if name not in seen:
            problems.append(f"{name} ran in no shard")
        elif len(seen[name]) > 1:
            problems.append(f"{name} ran in {len(seen[name])} shards: {', '.join(seen[name])}")
    for name in sorted(set(seen) - set(files)):
        problems.append(f"{name} ran but is not in {PLANTS_DIR}")

    # lib.sh's positive corpus check runs once per process, so it is in every
    # shard's tally; it is one check, and the total counts it once.
    passed = failed = 0
    fixture = (0, 0)
    for report in reports.values():
        passed += report["total"][0] - report["fixture"][0]
        failed += report["total"][1] - report["fixture"][1]
        fixture = (max(fixture[0], report["fixture"][0]), max(fixture[1], report["fixture"][1]))
    if fixture[1]:
        fixture = (0, 1)
    passed += fixture[0]
    failed += fixture[1]
    print(f"plant-select: {len(reports)} of {args.of} shard(s) reported, {len(seen)} of {len(files)} plant file(s) ran")
    print(f"{passed} passed, {failed} failed")
    if args.weights_out:
        with open(args.weights_out, "w") as out:
            out.write(f"# Seconds per plant file, from the shards' own reports. See {WEIGHTS_FILE}.\n")
            for name in sorted(seconds, key=os.fsencode):
                out.write(f"{max(seconds[name], 1)}\t{name}\n")
    for problem in problems:
        print(f"REFUSED  {problem}")
    if problems or failed:
        sys.exit(1)


def cmd_check_map(_args):
    files = plant_files()
    tracked = subprocess.run(
        ["git", "-C", str(ROOT), "ls-files", "-z"], capture_output=True, timeout=120, check=True
    ).stdout.decode("utf-8", "surrogateescape").split("\0")
    problems = []
    for rule in load_map():
        for pattern in rule["plants"]:
            if pattern != "*" and not any(fnmatch.fnmatchcase(f, pattern) for f in files):
                problems.append(f"rule {rule['name']}: plant pattern {pattern!r} names no plant")
        for pattern, regex in zip(rule["paths"], rule["regexes"]):
            if not any(regex.match(t) for t in tracked):
                problems.append(f"rule {rule['name']}: path pattern {pattern!r} names no tracked file")
    for problem in problems:
        print(f"REFUSED  {problem}")
    if problems:
        sys.exit(1)
    print("plant-select: every rule of the map names something real")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    shard = sub.add_parser("shard")
    shard.add_argument("shard")
    shard.set_defaults(run=cmd_shard)
    affected = sub.add_parser("affected")
    affected.add_argument("--base", required=True)
    affected.add_argument("--shard", required=True)
    affected.add_argument("--cap", required=True, type=int, help="seconds for one leg")
    affected.add_argument("--out")
    affected.set_defaults(run=cmd_affected)
    reports = sub.add_parser("reports")
    reports.add_argument("dir")
    reports.add_argument("--of", required=True, type=int)
    reports.add_argument("--weights-out")
    reports.set_defaults(run=cmd_reports)
    check_map = sub.add_parser("check-map")
    check_map.set_defaults(run=cmd_check_map)
    args = parser.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
