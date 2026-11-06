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
        print(f"{len(VERBS)} verbs, {total} tree reads, once each; a new process reads afresh")


if __name__ == "__main__":
    main()
