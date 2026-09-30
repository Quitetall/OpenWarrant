# SPDX-License-Identifier: Apache-2.0
import copy
import json
import subprocess
import sys
import unittest
from pathlib import Path

from evaluate import evaluate, unique_object


def session(participant="P1"):
    return {"schema": "oh.war/study-session-draft/v1", "participant": participant,
            "build_commit": "a" * 40, "inputs_sha256": "b" * 64,
            "host_os": "Linux", "scenario": "synthetic checker fixture, not a human session",
            "measurement_method": "synthetic monotonic seconds", "consent_confirmed": True,
            "assisted": False, "outcome": "completed", "failures": [], "elapsed_seconds": 670,
            "intervals": [{"start": 0, "end": 600, "category": "setup"},
                          {"start": 600, "end": 660, "category": "administration"},
                          {"start": 660, "end": 670, "category": "review"}]}


class EvaluationTests(unittest.TestCase):
    def test_exact_thresholds_and_fixed_denominator(self):
        self.assertFalse(evaluate([session()])["all_measurements_within_thresholds"])
        report = evaluate([session(p) for p in ("P1", "P2", "P3")])
        self.assertTrue(report["all_measurements_within_thresholds"])
        self.assertFalse(report["qualification_established"])
        self.assertEqual(report["results"][0]["seconds"]["review"], 10)

    def test_overrun_and_unknown_are_distinct(self):
        record = session()
        record["intervals"][1]["end"] = 661
        record["intervals"][2]["start"] = 661
        self.assertEqual(evaluate([record])["results"][0]["result"], "not_met")
        record["elapsed_seconds"] = None
        result = evaluate([record])["results"][0]
        self.assertEqual(result["result"], "unknown")
        self.assertNotIn("seconds", result)

    def test_gap_overlap_and_missing_consent_refuse(self):
        for value in (599, 601):
            record = session()
            record["intervals"][1]["start"] = value
            with self.assertRaisesRegex(ValueError, "gaps or overlap"):
                evaluate([record])
        record = session()
        record["consent_confirmed"] = False
        with self.assertRaisesRegex(ValueError, "consent"):
            evaluate([record])

    def test_assistance_failure_and_duplicate_remain_visible(self):
        record = session()
        record["assisted"] = True
        self.assertEqual(evaluate([record])["results"][0]["result"], "review_required")
        record["failures"] = ["facilitator took over"]
        self.assertEqual(evaluate([record])["results"][0]["result"], "not_met")
        with self.assertRaisesRegex(ValueError, "unique"):
            evaluate([record, copy.deepcopy(record)])

    def test_invalid_numbers_exclusion_and_duplicate_keys_refuse(self):
        for value in (True, float("nan"), float("inf"), -1, 10**1000):
            record = session()
            record["elapsed_seconds"] = value
            with self.assertRaises(ValueError):
                evaluate([record])
        record = session()
        record["intervals"][2]["category"] = "excluded"
        with self.assertRaisesRegex(ValueError, "reason"):
            evaluate([record])
        with self.assertRaisesRegex(ValueError, "duplicate"):
            json.loads('{"participant":"P1","participant":"P2"}', object_pairs_hook=unique_object)

    def test_cli_with_no_sessions_reports_missing_not_success(self):
        result = subprocess.run([sys.executable, str(Path(__file__).with_name("evaluate.py"))], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)["missing_participants"], ["P1", "P2", "P3"])


if __name__ == "__main__":
    unittest.main()
