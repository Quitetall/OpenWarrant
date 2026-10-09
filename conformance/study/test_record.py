# SPDX-License-Identifier: Apache-2.0
"""Synthetic operator data and controlled clock tests; not participant evidence."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

from evaluate import evaluate
from record import Recorder, MAX_EVENTS


def declared():
    return {"participant": "P1", "build_commit": "a" * 40, "inputs_sha256": "b" * 64,
            "scenario": "synthetic recorder control; no actual participant or consent",
            "consent_confirmed": True, "assisted": False, "failures": []}


def clock(*values):
    sequence = iter(values)
    return lambda: next(sequence)


class RecorderTests(unittest.TestCase):
    def test_actual_transitions_cover_time_and_cannot_qualify_a_study(self):
        r = Recorder(declared(), clock(10, 1_000_000_010, 2_000_000_010))
        r.command({"category": "review"})
        r.command({"finish": "completed"})
        record = r.record()
        self.assertEqual(record["intervals"], [{"start": 0, "end": 1, "category": "setup"}, {"start": 1, "end": 2, "category": "review"}])
        self.assertFalse(record["operator_claims_authenticated"])
        self.assertFalse(evaluate([record])["qualification_established"])
        self.assertFalse(evaluate([record])["all_measurements_within_thresholds"])
        self.assertEqual(r.events[-1]["offset_ns"], 2_000_000_000)

    def test_invalid_category_and_exclusion_never_invent_an_interval(self):
        r = Recorder(declared(), clock(0, 1_000_000_000))
        for command in ({"category": "hidden"}, {"category": "excluded"}, {"elapsed_seconds": 1}):
            with self.assertRaises(ValueError): r.command(command)
        self.assertEqual(len(r.events), 1)
        r.stop("invalid-input")
        self.assertEqual(r.record()["outcome"], "unknown")
        self.assertEqual(r.record()["intervals"][0]["category"], "setup")

    def test_exclusion_and_assistance_remain_visible(self):
        r = Recorder(declared(), clock(0, 1_000_000_000, 2_000_000_000, 3_000_000_000))
        r.command({"category": "excluded", "reason": "synthetic interruption"})
        r.command({"assisted": True})
        with self.assertRaises(ValueError): r.command({"assisted": False})
        r.command({"finish": "completed"})
        self.assertTrue(r.record()["assisted"])
        self.assertEqual(r.record()["intervals"][1]["reason"], "synthetic interruption")
        self.assertEqual(evaluate([r.record()])["results"][0]["result"], "review_required")

    def test_unavailable_clock_and_zero_elapsed_stay_unknown(self):
        for values in ((10, 9), (10, 10)):
            r = Recorder(declared(), clock(*values))
            r.stop("test")
            self.assertIsNone(r.record()["elapsed_seconds"])
            self.assertEqual(evaluate([r.record()])["results"][0]["result"], "unknown")

    def test_event_limit_keeps_room_to_record_stop(self):
        r = Recorder(declared(), clock=lambda: 0)
        for _ in range(MAX_EVENTS - 2): r.command({"category": "waiting"})
        with self.assertRaisesRegex(ValueError, "limit"): r.command({"category": "setup"})
        r.stop("event-limit")
        self.assertEqual(len(r.events), MAX_EVENTS)
        self.assertEqual(r.events[-1]["kind"], "stopped")

    def test_metadata_rejects_private_extra_fields_and_missing_consent(self):
        for value in ({**declared(), "email": "not collected"}, {**declared(), "consent_confirmed": False}):
            with self.assertRaises(ValueError): Recorder(value)

    def test_native_cli_eof_retains_measured_interruption_and_refuses_overwrite(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            meta, out = root / "metadata.json", root / "session.json"
            meta.write_text(json.dumps(declared()))
            args = [sys.executable, str(Path(__file__).with_name("record.py")), "--metadata", str(meta), "--out", str(out), "--max-seconds", "2"]
            process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            time.sleep(0.1)
            result, error = process.communicate(b'{"category":"review"}\n', timeout=5)
            self.assertEqual(process.returncode, 1, error)
            answer = json.loads(result)
            self.assertFalse(answer["qualification_established"])
            record = json.loads(out.read_bytes())
            self.assertGreater(record["elapsed_seconds"], 0)
            self.assertEqual(record["termination"], "end-of-input")
            self.assertEqual(record["outcome"], "unknown")
            events = root / record["events"]["file"]
            event_bytes = events.read_bytes()
            self.assertEqual(hashlib.sha256(event_bytes).hexdigest(), record["events"]["sha256"])
            before = out.read_bytes()
            refused = subprocess.run(args, input=b'', capture_output=True, timeout=5)
            self.assertEqual(refused.returncode, 2)
            self.assertEqual(out.read_bytes(), before)
            self.assertEqual(events.read_bytes(), event_bytes)

    def test_native_cli_sigterm_retains_unknown_interruption(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); meta = root / "meta.json"; out = root / "out.json"
            meta.write_text(json.dumps(declared()))
            process = subprocess.Popen([sys.executable, str(Path(__file__).with_name("record.py")), "--metadata", str(meta), "--out", str(out)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                deadline = time.monotonic() + 3
                ledger = root / "out.json.events.jsonl"
                while time.monotonic() < deadline:
                    if ledger.exists() and ledger.stat().st_size > 0:
                        break
                    time.sleep(0.01)
                else:
                    self.fail("recorder did not start")
                # Allow the handler to be installed after the started event.
                time.sleep(0.05)
                process.terminate()
                result, error = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 1, error)
                record = json.loads(out.read_bytes())
                self.assertEqual(record["termination"], "signal-interruption")
                self.assertEqual(record["outcome"], "unknown")
                self.assertFalse(json.loads(result)["qualification_established"])
            finally:
                if process.poll() is None:
                    process.kill(); process.communicate(timeout=5)

    def test_unavailable_clock_and_failure_bound_preserve_unknown(self):
        def unavailable(): raise OSError("synthetic clock failure")
        r = Recorder(declared(), clock=lambda: 0)
        r.clock = unavailable
        with self.assertRaisesRegex(ValueError, "clock unavailable"):
            r.command({"category": "review"})
        r.stop("invalid-input")
        self.assertIsNone(r.record()["elapsed_seconds"])
        r = Recorder({**declared(), "failures": ["synthetic"] * 100}, clock=lambda: 0)
        with self.assertRaisesRegex(ValueError, "limit"):
            r.command({"failure": "extra"})
        self.assertEqual(len(r.events), 1)

    def test_native_cli_partial_input_deadline_is_bounded_and_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary); meta = root / "meta.json"; out = root / "out.json"
            meta.write_text(json.dumps(declared()))
            process = subprocess.Popen([sys.executable, str(Path(__file__).with_name("record.py")), "--metadata", str(meta), "--out", str(out), "--max-seconds", "0.1"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            process.stdin.write(b'{"category":'); process.stdin.flush()
            process.wait(timeout=5)
            process.stdin.close(); process.stdout.close(); process.stderr.close()
            self.assertEqual(process.returncode, 1)
            self.assertEqual(json.loads(out.read_bytes())["termination"], "session-deadline")


if __name__ == "__main__": unittest.main()
