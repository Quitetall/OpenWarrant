#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Bounded controls for the draft adapter, without Cargo or model calls."""

import importlib.util
import pathlib
import signal
import subprocess
import sys
import tempfile
import time
import unittest


SPEC = importlib.util.spec_from_file_location(
    "doctor_gate", pathlib.Path(__file__).with_name("doctor-gate.py"),
)
ADAPTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ADAPTER)


def live(pid):
    state = subprocess.run(
        ["ps", "-o", "stat=", "-p", str(pid)], capture_output=True,
        text=True, check=False,
    ).stdout.strip()
    return bool(state) and not state.startswith("Z")


class AdapterControls(unittest.TestCase):
    def test_pass_and_failure_propagate(self):
        with tempfile.TemporaryDirectory(prefix="ow97-adapter-exit-") as source:
            for status in [0, 19]:
                self.assertEqual(
                    ADAPTER.run_child([sys.executable, "-c", f"raise SystemExit({status})"], source, 3),
                    (status, False),
                )

    def test_timeout_stops_term_ignoring_descendant_and_preserves_unrelated_process(self):
        with tempfile.TemporaryDirectory(prefix="ow97-adapter-timeout-") as source:
            root = pathlib.Path(source)
            sentinel = subprocess.Popen(
                [sys.executable, "-c", "import time; time.sleep(30)"],
                start_new_session=True,
            )
            try:
                child = (
                    "import os,pathlib,signal,time; "
                    "signal.signal(signal.SIGTERM,signal.SIG_IGN); "
                    "pathlib.Path('descendant.pid').write_text(str(os.getpid())); "
                    "time.sleep(30)"
                )
                parent = (
                    f"import pathlib,subprocess,sys,time; subprocess.Popen([sys.executable,'-c',{child!r}]); "
                    "time.sleep(30)"
                )
                started = time.monotonic()
                self.assertEqual(ADAPTER.run_child([sys.executable, "-c", parent], source, 1), (124, True))
                self.assertLess(time.monotonic() - started, 5)
                descendant = int((root / "descendant.pid").read_text())
                for _ in range(30):
                    if not live(descendant):
                        break
                    time.sleep(0.05)
                self.assertFalse(live(descendant), "TERM-ignoring descendant still executes")
                self.assertIsNone(sentinel.poll(), "unrelated process was stopped")
            finally:
                sentinel.terminate()
                sentinel.wait(timeout=3)

    def test_parent_success_does_not_leave_its_background_member_running(self):
        with tempfile.TemporaryDirectory(prefix="ow97-adapter-background-") as source:
            root = pathlib.Path(source)
            child = (
                "import os,pathlib,time; "
                "pathlib.Path('background.pid').write_text(str(os.getpid())); time.sleep(30)"
            )
            parent = (
                f"import pathlib,subprocess,sys,time; subprocess.Popen([sys.executable,'-c',{child!r}]); "
                "\nwhile not pathlib.Path('background.pid').exists(): time.sleep(0.01)"
            )
            self.assertEqual(ADAPTER.run_child([sys.executable, "-c", parent], source, 3), (0, False))
            pid = int((root / "background.pid").read_text())
            for _ in range(30):
                if not live(pid):
                    break
                time.sleep(0.05)
            self.assertFalse(live(pid), "successful parent left work running")


if __name__ == "__main__":
    unittest.main()
