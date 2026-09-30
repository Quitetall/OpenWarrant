#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observed Linux runner boundary; no signature or human assurance claim."""
import pathlib
import subprocess
import tempfile

runner = pathlib.Path(__file__).resolve().with_name("authority-agent-sandbox.sh")
with tempfile.TemporaryDirectory(prefix="ow-authority-isolation-") as directory:
    root = pathlib.Path(directory)
    work = root / "work"
    work.mkdir()
    protected = root / "authority"
    protected.mkdir()
    state = protected / "state.json"
    state.write_bytes(b"original")
    result = subprocess.run(
        [str(runner), str(work), "--", "/bin/sh", "-c",
         'printf work > /work/result; test ! -e "$1" || exit 8; '
         'if printf attack > "$1" 2>/dev/null; then exit 9; fi; '
         'test -z "${SSH_AUTH_SOCK-}"', "sh", str(state)],
        capture_output=True, text=True, timeout=20,
    )
    if result.returncode:
        raise SystemExit(f"UNKNOWN/FAIL sandbox execution ({result.returncode}): {result.stderr}")
    assert (work / "result").read_bytes() == b"work"
    assert state.read_bytes() == b"original"
    print("PASS task write allowed; outside authority state unavailable and unchanged; signing socket absent")
