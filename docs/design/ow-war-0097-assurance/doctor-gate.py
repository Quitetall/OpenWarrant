#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run doctor acceptance fixtures with fresh build output outside the subject.

Review candidate only; not yet registered or bound to an approved Warrant.
The caller supplies the subject as the current working directory. The gate
runner owns the deadline and binds the resulting streams to that subject.
"""

import json
import pathlib
import subprocess
import sys
import tempfile


def main():
    source = pathlib.Path.cwd()
    if not (source / "Cargo.lock").is_file():
        print("doctor-gate: unavailable source prerequisite Cargo.lock", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory(prefix="ow-doctor-gate-build-") as temporary:
        argv = [
            "cargo", "+1.97.1", "test", "--frozen", "--target-dir",
            str(pathlib.Path(temporary) / "target"), "-p", "openwarrant-cli",
            "--test", "doctor_cli",
        ]
        print("doctor-gate argv " + json.dumps(argv), flush=True)
        try:
            return subprocess.run(argv, cwd=source, check=False).returncode
        except FileNotFoundError:
            print("doctor-gate: unavailable prerequisite cargo executable", file=sys.stderr)
            return 127


if __name__ == "__main__":
    sys.exit(main())
