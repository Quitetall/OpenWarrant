# SPDX-License-Identifier: Apache-2.0
"""Exercise the real one-shot progress snapshot, not a timing-sensitive mock of its evaluator."""
import collections
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    war = str(Path(sys.argv[1]).resolve())
    index = Path(subprocess.check_output(
        ["git", "rev-parse", "--git-path", "index"], text=True).strip())
    before = index.read_bytes()
    with tempfile.TemporaryDirectory(prefix="ow-overview-reads-") as scratch:
        trace = Path(scratch) / "trace.jsonl"
        env = {**os.environ, "GIT_TRACE2_EVENT": str(trace)}
        result = subprocess.run([war, "overview", "--snapshot", "--json"], env=env,
                                capture_output=True, check=True)
        body = json.loads(result.stdout)
        assert body["result"]["schema"] == "oh.war/progress-view/v1"
        reads = collections.Counter()
        for line in trace.read_text().splitlines():
            event = json.loads(line)
            args = event.get("argv", [])
            if event.get("event") == "start" and (
                    "--others" in args or "--name-only" in args):
                reads[tuple(args)] += 1
        assert reads, "control did not observe any actual Git tree reads"
        repeated = {str(args): count for args, count in reads.items() if count != 1}
        assert not repeated, f"overview repeated identical tree reads: {repeated}"
        assert index.read_bytes() == before, "read-only overview changed index bytes"
        print(f"overview used {sum(reads.values())} distinct tree reads, once each; index unchanged")


if __name__ == "__main__":
    main()
