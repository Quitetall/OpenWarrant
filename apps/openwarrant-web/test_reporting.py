# SPDX-License-Identifier: Apache-2.0
import copy
import unittest
from reporting import render, eligible


class ReportTests(unittest.TestCase):
    def fixture(self):
        r = {"attempt_id": "attempt", "warrant_id": "warrant", "source_sha256": "a" * 64,
             "sequence": 2, "work_state": "completed", "execution_state": "stopped",
             "result_revision": "b" * 40, "policy": {"checks": [["test"]], "source_sha256": "a" * 64},
             "checks": [{"argv": ["test"], "exit_code": 0}], "notes": "<script>bad()</script>",
             "next_steps": ["Review"], "cause": "Work finished"}
        return {"attempt": r}, {"warrant": copy.deepcopy(r["policy"]), "waiting": {"source_sha256": "c" * 64}}

    def test_deterministic_scoped_overview_and_escaped_offline_html(self):
        records, inventory = self.fixture()
        report = render(records, inventory, "attempt", "FINISHED", "minimal")
        self.assertEqual(report, render(records, inventory, "attempt", "FINISHED", "minimal"))
        self.assertEqual(report["text"], "FINISHED\nProgress: /#attempt=attempt\n")
        self.assertEqual(report["progress"]["completed"], 1)
        self.assertEqual(report["progress"]["total"], 2)
        self.assertEqual(report["progress"]["pending"], ["waiting"])
        self.assertNotIn("<script>", report["html"])
        self.assertIn("&lt;script&gt;", report["html"])
        self.assertFalse(report["qualified"])
        self.assertLess(report["html"].index("Implementation notes"), report["html"].index("<details>"))
        self.assertIn("<li>Review</li>", report["html"])
        self.assertIn("<li>waiting</li>", report["html"])
        inventory["warrant"]["source_sha256"] = "d" * 64
        changed = render(records, inventory, "attempt")
        self.assertEqual(changed["progress"]["completed"], 0)
        self.assertNotEqual(report["snapshot_sha256"], changed["snapshot_sha256"])

    def test_changed_policy_preserves_history_but_cannot_discharge_current_work(self):
        records, inventory = self.fixture()
        original = copy.deepcopy(records)
        for change in ({"checks": [["new-test"]]}, {"base_commit": "e" * 40},
                       {"dependencies": ["new-dependency"]}, {"verified_start": True}):
            with self.subTest(change=change):
                policy = {**inventory["warrant"], **change}
                self.assertFalse(eligible(records["attempt"], policy))
                report = render(records, {"warrant": policy}, "attempt", detail="minimal")
                self.assertEqual(report["progress"]["completed"], 0)
                self.assertFalse(report["current_policy_eligible"])
                self.assertIn("historical result only", report["text"])
                self.assertIn("historical result only", report["html"])
                self.assertEqual(records, original)

    def test_incomplete_or_unestablished_results_never_emit_signal(self):
        records, inventory = self.fixture()
        for changes in ({"sequence": 1}, {"work_state": "failed"}, {"work_state": "blocked"},
                        {"execution_state": "running"}, {"execution_state": "unknown"},
                        {"result_revision": None}, {"result_revision": "b" * 41}, {"checks": []},
                        {"checks": [{"argv": ["test"], "exit_code": False}]},
                        {"checks": [{"argv": ["other"], "exit_code": 0}]}):
            with self.subTest(changes=changes):
                candidate = copy.deepcopy(records)
                candidate["attempt"].update(changes)
                report = render(candidate, inventory, "attempt", "FINISHED", "minimal")
                self.assertIsNone(report["completion_signal"])
                self.assertTrue(report["text"].startswith("WORK_INCOMPLETE\n"))
                self.assertEqual(report["progress"]["completed"], 0)


if __name__ == "__main__":
    unittest.main()
