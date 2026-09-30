# SPDX-License-Identifier: Apache-2.0
"""Bounded filesystem refusal before reading workflow state and report inputs."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from server import Refusal, read_file


class FileReadTests(unittest.TestCase):
    def test_regular_file_limit_remains_exact(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "record"
            path.write_bytes(b"abcd")
            self.assertEqual(read_file(path, 4), b"abcd")
            with self.assertRaises(Refusal) as caught:
                read_file(path, 3)
            self.assertEqual(caught.exception.status, 413)

    def test_directory_refuses_as_nonregular(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(Refusal) as caught:
                read_file(Path(tmp))
            self.assertEqual(caught.exception.status, 409)

    @unittest.skipUnless(hasattr(os, "mkfifo"), "POSIX FIFO required")
    def test_fifo_without_writer_refuses_before_blocking_open(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "record"
            os.mkfifo(path)
            code = """import sys
from server import Refusal, read_file
try:
    read_file(sys.argv[1])
except Refusal as error:
    print(error.status, error.message)
else:
    raise SystemExit('nonregular input was accepted')
"""
            try:
                result = subprocess.run([sys.executable, "-c", code, str(path)],
                                        cwd=Path(__file__).parent, capture_output=True,
                                        text=True, timeout=3)
            except subprocess.TimeoutExpired:
                self.fail("FIFO read blocked instead of refusing nonregular input")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), "409 Regular file required")
