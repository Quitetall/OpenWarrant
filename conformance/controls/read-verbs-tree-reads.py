# SPDX-License-Identifier: Apache-2.0
"""One-shot read verbs ask each tree question once, and only within one process."""
import collections
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

VERBS = (
    ["view", "context", "--index"],
    ["plan", "frontier", "--all"],
    ["plan", "frontier"],
    ["plan", "estimate"],
)


def tree_reads(war, verb, trace):
    trace.unlink(missing_ok=True)
    env = {**os.environ, "GIT_TRACE2_EVENT": str(trace)}
    subprocess.run([war, *verb], env=env, capture_output=True, check=True)
    reads = collections.Counter()
    if trace.exists():
        for line in trace.read_text().splitlines():
            event = json.loads(line)
            args = event.get("argv", [])
            if event.get("event") == "start" and (
                    "--others" in args or "--name-only" in args):
                reads[tuple(args)] += 1
    return reads


def main():
    war = str(Path(sys.argv[1]).resolve())
    index = Path(subprocess.check_output(
        ["git", "rev-parse", "--git-path", "index"], text=True).strip())
    # A tracked file whose stat data moved while its bytes did not is what
    # makes `git status` and `git diff` rewrite the index when they may: with
    # one planted, an index write by a read verb happens every time, not on a
    # lucky timing.
    tracked = Path(subprocess.check_output(
        ["git", "ls-files", "-z", "--", "*.md"], text=True).split("\0")[0])
    assert tracked.is_file(), "control found no tracked file to touch"
    os.utime(tracked, ns=(tracked.stat().st_atime_ns, tracked.stat().st_mtime_ns + 2_000_000_000))
    before = index.read_bytes()
    with tempfile.TemporaryDirectory(prefix="ow-read-verbs-") as scratch:
        trace = Path(scratch) / "trace.jsonl"
        total = 0
        for verb in VERBS:
            reads = tree_reads(war, verb, trace)
            assert reads, f"{verb}: control did not observe any actual Git tree reads"
            repeated = {str(a): n for a, n in reads.items() if n != 1}
            assert not repeated, f"{verb} repeated identical tree reads: {repeated}"
            total += sum(reads.values())
            # The memo is this process's: a persistent one would read nothing here.
            assert tree_reads(war, verb, trace), f"{verb}: a new process did not read the tree"
        assert index.read_bytes() == before, "a read verb changed index bytes"
        # The refusal: the same question asked of git without the guard does
        # rewrite it, so the equality above is the guard at work.
        subprocess.run(["git", "diff", "--quiet", "HEAD", "--", str(tracked)], check=False)
        assert index.read_bytes() != before, "the planted stat change did not make git refresh the index"
        print(f"{len(VERBS)} verbs, {total} tree reads, once each; a new process reads afresh")


if __name__ == "__main__":
    main()
