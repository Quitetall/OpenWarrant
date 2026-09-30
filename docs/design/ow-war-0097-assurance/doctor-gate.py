#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run doctor acceptance fixtures with fresh build output outside the subject.

Review candidate only; not yet registered or bound to an approved Warrant.
The caller supplies the subject as the current working directory. The gate
runner owns the deadline and binds the resulting streams to that subject.
"""

import json
import pathlib
import os
import signal
import subprocess
import sys
import tempfile


def stop_group(child):
    """Signal only this child's group; detached descendants are outside it."""
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        child.wait(timeout=1)
    except subprocess.TimeoutExpired:
        pass
    # A descendant may ignore TERM even when the leader exits. Do not return
    # early just because wait() observed the leader's exit.
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        child.wait(timeout=1)
    except subprocess.TimeoutExpired as error:
        raise RuntimeError("UNKNOWN: child stop could not be established") from error


def run_child(argv, source, timeout_seconds):
    """Return (exit code, timed out), fencing this command in its own session."""
    child = subprocess.Popen(argv, cwd=source, start_new_session=True)
    try:
        try:
            return child.wait(timeout=timeout_seconds), False
        except subprocess.TimeoutExpired:
            return 124, True
    finally:
        stop_group(child)


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
            # The proposed definition gives this adapter 900 seconds. Leave
            # 60 seconds for cleanup before the outer runner's hard deadline.
            status, timed_out = run_child(argv, source, 840)
            if timed_out:
                print("doctor-gate: UNKNOWN behavioral observation; child deadline exceeded", file=sys.stderr)
            return status
        except (OSError, RuntimeError) as error:
            print(f"doctor-gate: unavailable cargo execution prerequisite: {error}", file=sys.stderr)
            return 127


if __name__ == "__main__":
    sys.exit(main())
