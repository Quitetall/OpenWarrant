#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Read four frozen source files and compare approximations; never call a model."""
import argparse
import hashlib
import json
import os
import subprocess
import sys

SOURCES = (
    "docs/sas/WAR_Software_Architecture_Specification.md",
    "docs/sas/generated/NORMATIVE.md",
    "crates/openwarrant-cli/src/check.rs",
    "docs/warrants/OW-WAR-0064/atoms/10-intent.md",
)

def run(argv, data=None):
    return subprocess.run(argv, input=data, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=True, timeout=30,
                          env={**os.environ, "LC_ALL": "C"}).stdout

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", required=True,
                        help="Git revision containing all four required sources")
    parser.add_argument("--path", choices=SOURCES, help="reproduce one named row")
    args = parser.parse_args()
    commit = run(["git", "rev-parse", "--verify", "--end-of-options",
                  args.commit + "^{commit}"]).decode("ascii").strip()
    rows = []
    for path in (args.path,) if args.path else SOURCES:
        raw = run(["git", "cat-file", "blob", commit + ":" + path])
        byte_count = int(run(["wc", "-c"], raw))
        word_count = int(run(["wc", "-w"], raw))
        if byte_count != len(raw):
            raise ValueError("wc byte count disagrees with captured source length")
        rows.append({"path": path, "source_sha256": hashlib.sha256(raw).hexdigest(),
                     "source_command": ["git", "cat-file", "blob", commit + ":" + path],
                     "bytes_command": ["wc", "-c"],
                     "words_command": ["wc", "-w"],
                     "counter_input": "captured source blob on stdin",
                     "bytes": byte_count, "words": word_count,
                     "bytes_div_4_ceil": (byte_count + 3) // 4,
                     "words_times_13_div_10_ceil": (13 * word_count + 9) // 10})
    print(json.dumps({"source_commit": commit, "locale": "C",
                      "byte_count_command": ["wc", "-c"],
                      "word_count_command": ["wc", "-w"],
                      "rounding": "ceiling for both compared estimates",
                      "tokenizer_called": False, "assurance_granted": False,
                      "sources": rows}, indent=2))

if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print("measurement unavailable: " + str(error), file=sys.stderr)
        sys.exit(2)
