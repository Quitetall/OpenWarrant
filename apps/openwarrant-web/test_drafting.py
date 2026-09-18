# SPDX-License-Identifier: Apache-2.0
"""Synthetic adapter over real HTTP/process/SDK; no model qualification claim."""
import hashlib
import json
import os
import signal
import sys
import time
import uuid

from test_workflow import REPO, WorkflowTests


class DraftingTests(WorkflowTests):
    def start(self):
        if not hasattr(self, "config"):
            self.harness = self.root / "drafter.py"
            self.marker = self.root / "calls.txt"
            self.harness.write_text(
                "import json,sys,pathlib\n"
                "r=json.load(sys.stdin)\n"
                "assert r['skill']=='war spec' and r['context'][0]['role']=='skill'\n"
                f"p=pathlib.Path({str(self.marker)!r});p.write_text(p.read_text()+'x' if p.exists() else 'x')\n"
                "print(json.dumps({'schema':'oh.war/drafting-result/v1','fields':{'title':'Draft title',"
                "'outcome':r['prompt'],'scope':'Only the requested outcome; add positive and refusal tests.',"
                "'context':'Pinned skill: '+r['context'][0]['sha256']}}))\n")
            skill = ".claude/skills/war-spec/SKILL.md"
            self.config = {
                "schema": "oh.war/drafting-config/v1", "argv": [sys.executable, str(self.harness)],
                "backend": "synthetic adapter; no model", "sandbox": "read-only-repository",
                "cost_mode": "free", "spend_limit_usd": 10, "timeout_seconds": 2,
                "context": [{"path": skill, "sha256": hashlib.sha256((REPO / skill).read_bytes()).hexdigest(), "role": "skill"}]}
        self.config_path = self.root / "drafting.json"
        self.config_path.write_text(json.dumps(self.config))
        self.server_args = ["--drafting-config", str(self.config_path)]
        super().start()

    def request(self, prompt="Add a changelog"):
        fields = {"request_id": str(uuid.uuid4()), "prompt": prompt}
        status, record = self.call("/api/drafting", "POST", fields)
        self.assertEqual(status, 202, record)
        return fields

    def result(self, id):
        for _ in range(250):
            status, record = self.call("/api/drafting/" + id)
            self.assertEqual(status, 200, record)
            if record["state"] != "running":
                return record
            time.sleep(0.02)
        self.fail("draft did not terminate")

    def test_generated_source_is_unsaved_then_saved_through_sdk_and_replay_is_safe(self):
        fields = self.request()
        r = self.result(fields["request_id"])
        self.assertEqual(r["state"], "ready", r)
        self.assertEqual(r["result"]["fields"]["outcome"], fields["prompt"])
        self.assertIn("oh.war/document/1.0.0-rc.3", r["result"]["source"])
        self.assertFalse(r["saved"])
        self.assertFalse(r["qualified"])
        self.assertEqual(self.call("/api/warrants")[1]["warrants"], [])
        self.stop(); self.start()
        self.assertEqual(self.call("/api/drafting", "POST", fields)[1], r)
        self.assertEqual(self.marker.read_text(), "x")
        self.assertEqual(self.call("/api/drafting", "POST", {**fields, "prompt": "Different"})[0], 409)
        status, saved = self.call("/api/warrants", "POST", {"id": fields["request_id"], **r["result"]["fields"]})
        self.assertEqual(status, 201, saved)
        self.assertEqual(saved["source"], r["result"]["source"])
        self.assertFalse(saved["qualified"])

    def test_cost_and_context_refusals_precede_dispatch(self):
        self.stop(); self.config["cost_mode"] = "unknown"; self.start()
        fields = {"request_id": str(uuid.uuid4()), "prompt": "Draft work"}
        self.assertEqual(self.call("/api/drafting", "POST", fields)[0], 403)
        self.assertFalse(self.marker.exists())
        self.stop(); self.config["spend_limit_usd"] = None; self.start()
        self.assertEqual(self.call("/api/drafting", "POST", fields)[0], 202)
        self.assertIsNone(self.result(fields["request_id"])["cost_usd"])
        self.stop(); self.config["context"][0]["sha256"] = "0" * 64; self.start()
        fields["request_id"] = str(uuid.uuid4())
        self.assertEqual(self.call("/api/drafting", "POST", fields)[0], 409)
        self.assertEqual(self.marker.read_text(), "x")

    def test_invalid_authority_fields_and_malformed_output_never_save(self):
        for output in ("not JSON", json.dumps({"schema": "oh.war/drafting-result/v1", "fields": {"title": "x", "qualified": True}})):
            self.harness.write_text("import sys;sys.stdin.read();print(" + repr(output) + ")")
            fields = self.request()
            self.assertEqual(self.result(fields["request_id"])["state"], "failed")
            self.assertEqual(self.call("/api/warrants")[1]["warrants"], [])

    def test_timeout_stays_unknown_and_blocks_replacement(self):
        self.harness.write_text("import time;time.sleep(20)")
        fields = self.request()
        other = {"request_id": str(uuid.uuid4()), "prompt": "Another draft"}
        self.assertEqual(self.call("/api/drafting", "POST", other)[0], 409)
        self.assertEqual(self.result(fields["request_id"])["state"], "unknown")
        self.stop(); self.start()
        self.assertEqual(self.call("/api/drafting", "POST", other)[0], 409)
        self.assertEqual(self.call("/api/drafting", "POST", fields)[1]["state"], "unknown")

    def test_interrupted_initial_record_survives_restart_as_unknown(self):
        pidfile = self.root / "owned-harness.pid"
        self.harness.write_text("import os,pathlib,time;pathlib.Path(" + repr(str(pidfile)) + ").write_text(str(os.getpid()));time.sleep(20)")
        fields = self.request()
        for _ in range(100):
            if pidfile.exists():
                break
            time.sleep(0.02)
        self.assertTrue(pidfile.exists())
        pid = int(pidfile.read_text())
        self.stop()
        try:
            self.start()
            record = self.result(fields["request_id"])
            self.assertEqual(record["state"], "unknown")
            self.assertEqual(record["sequence"], 1)
            self.assertEqual(self.call("/api/drafting", "POST", fields)[1], record)
            other = {"request_id": str(uuid.uuid4()), "prompt": "Replacement"}
            self.assertEqual(self.call("/api/drafting", "POST", other)[0], 409)
        finally:
            # Only the synthetic child launched by this test; no service-wide kill.
            try:
                os.killpg(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    def test_output_limit_and_authentication(self):
        self.assertEqual(self.call("/api/drafting", headers={"Authorization": "Bearer wrong"})[0], 401)
        self.assertEqual(self.call("/api/drafting", "POST", {"request_id": str(uuid.uuid4()), "prompt": "x", "qualified": True})[0], 400)
        self.harness.write_text("import sys;sys.stdin.read();sys.stdout.write('x'*2000000)")
        fields = self.request()
        self.assertEqual(self.result(fields["request_id"])["state"], "unknown")
        self.assertEqual(self.call("/api/warrants")[1]["warrants"], [])


if __name__ == "__main__":
    import unittest
    unittest.main()
