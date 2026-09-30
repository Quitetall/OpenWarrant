# SPDX-License-Identifier: Apache-2.0
"""Bounded process transport; the configured harness owns sandboxing."""
import os
import signal
import subprocess
import threading
import time

LIMIT = 1024 * 1024

def argv(value):
    return (
        isinstance(value, list)
        and 0 < len(value) <= 64
        and all(
            isinstance(s, str) and s and len(s) <= 8192 and "\0" not in s for s in value
        )
    )


def bounded_command(args, cwd, stdin, deadline, capture):
    # Pipes cap output in memory. The configured harness must contain descendants;
    # process groups alone cannot fence escaped processes or remote jobs.
    if time.monotonic() >= deadline:
        raise TimeoutError("Execution time limit reached")
    proc = subprocess.Popen(
        args,
        cwd=cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    output = [bytearray(), bytearray()]
    overflow = threading.Event()

    def drain(stream, dest):
        while chunk := stream.read(8192):
            if len(dest) + len(chunk) > LIMIT:
                overflow.set()
                break
            dest.extend(chunk)
        stream.close()

    readers = [
        threading.Thread(target=drain, args=(stream, dest), daemon=True)
        for stream, dest in zip((proc.stdout, proc.stderr), output)
    ]
    for t in readers:
        t.start()

    def feed():
        try:
            proc.stdin.write(stdin)
        except (BrokenPipeError, OSError):
            pass
        finally:
            proc.stdin.close()

    writer = threading.Thread(target=feed, daemon=True)
    writer.start()
    try:
        while proc.poll() is None:
            if time.monotonic() >= deadline or overflow.is_set():
                raise TimeoutError(
                    "Execution limit reached; writer fencing remains unknown"
                )
            time.sleep(0.02)
        for t in readers:
            t.join(timeout=0.2)
        if overflow.is_set() or any(t.is_alive() for t in readers):
            raise TimeoutError(
                "Process output overflow or open inherited pipes; execution unknown"
            )
        try:
            os.killpg(proc.pid, 0)
        except ProcessLookupError:
            pass
        else:
            raise TimeoutError(
                "Harness left live descendants; writer fencing remains unknown"
            )
        return proc.returncode, bytes(output[0]), bytes(output[1])
    finally:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        proc.wait(timeout=3)
        writer.join(timeout=1)
        for t in readers:
            t.join(timeout=0.2)
        for name, data in zip(("stdout", "stderr"), output):
            capture[name] = bytes(data[:2048]).decode(errors="replace")
            capture[name + "_truncated"] = len(data) > 2048 or overflow.is_set()
        capture["capture_incomplete"] = overflow.is_set() or any(
            t.is_alive() for t in readers
        )

