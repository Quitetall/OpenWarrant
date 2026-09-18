# SPDX-License-Identifier: Apache-2.0
"""Real HTTP/process/Git execution seam; synthetic harness, no model calls."""

import json
import os
import subprocess
import sys
import time
import uuid
from pathlib import Path

from test_workflow import APP, WAR, WorkflowTests


class ExecutionTests(WorkflowTests):
    def start(self):
        self.repo = self.root / "repo"
        if not self.repo.exists():
            self.repo.mkdir()
            self.git("init", "-q")
            self.git("config", "user.email", "fixture@example.invalid")
            self.git("config", "user.name", "Fixture")
            (self.repo / "README.md").write_text("fixture\n")
            self.git("add", ".")
            self.git("commit", "-qm", "initial")
            self.base = self.git("rev-parse", "HEAD").strip()
            self.draft_id = str(uuid.uuid4())
            self.harness = self.root / "harness.py"
            self.harness.write_text("""import json,sys,subprocess,pathlib
r=json.load(sys.stdin)
p=pathlib.Path(r['worktree'])
(p/'result.txt').write_text('implemented\\n')
subprocess.run(['git','add','result.txt'],cwd=p,check=True,stdout=sys.stderr)
subprocess.run(['git','commit','-qm','fixture implementation'],cwd=p,check=True,stdout=sys.stderr)
print(json.dumps({'schema':'oh.war/execution-result/v1','attempt_id':r['attempt_id'],'source_sha256':r['source_sha256'],'work_state':'completed','notes':'Fixture implemented file.','next_steps':['Independent review']}))
""")
            self.config = {
                "schema": "oh.war/execution-config/v1",
                "argv": [sys.executable, str(self.harness)],
                "sandbox": "harness-worktree-only",
                "cost_mode": "free",
                "spend_limit_usd": 10,
                "timeout_seconds": 5,
                "repair_cycles": 3,
                "warrants": {},
            }
        self.config_path = self.root / "execution.json"
        self.config_path.write_text(json.dumps(self.config))
        self.session = self.root / ("session-" + str(uuid.uuid4()) + ".json")
        self.proc = subprocess.Popen(
            [
                sys.executable,
                str(APP),
                "--war",
                WAR,
                "--repo",
                str(self.repo),
                "--state",
                str(self.root / getattr(self, "state_name", "state")),
                "--port",
                "0",
                "--session-file",
                str(self.session),
                "--execution-config",
                str(self.config_path),
                *getattr(self, "report_args", []),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        for _ in range(200):
            if self.session.exists():
                break
            if self.proc.poll() is not None:
                self.fail(self.proc.stderr.read().decode())
            time.sleep(0.02)
        self.assertTrue(self.session.exists())
        info = json.loads(self.session.read_text())
        self.url = info["url"]
        self.token = info["token"]

    def git(self, *args):
        return subprocess.check_output(
            ["git", *args], cwd=self.repo, text=True, stderr=subprocess.DEVNULL
        )

    def eligible(self):
        d = {**self.draft(), "id": self.draft_id}
        status, record = self.call("/api/warrants", "POST", d)
        self.assertEqual(status, 201, record)
        self.config["warrants"][d["id"]] = {
            "source_sha256": record["source_sha256"],
            "base_commit": self.base,
            "verified_start": False,
            "dependencies": [],
            "checks": [
                [
                    sys.executable,
                    "-c",
                    "from pathlib import Path; assert Path('result.txt').read_text() == 'implemented\\n'",
                ]
            ],
        }
        self.stop()
        self.start()
        return record

    def wait_run(self, id):
        for _ in range(300):
            status, r = self.call("/api/runs/" + id)
            self.assertEqual(status, 200, r)
            if r["execution_state"] != "running":
                return r
            time.sleep(0.02)
        self.fail("attempt never finished")

    def test_work_report_is_persisted_read_only_and_configurable(self):
        r = self.eligible()
        self.stop()
        self.report_args = ["--completion-word", "DONE_TEST", "--report-detail", "minimal"]
        self.start()
        status, run = self.call("/api/runs", "POST", {"warrant_id": r["id"], "source_sha256": r["source_sha256"]})
        self.assertEqual(status, 202)
        self.assertEqual(self.wait_run(run["attempt_id"])["work_state"], "completed")
        route = "/api/runs/" + run["attempt_id"] + "/report"
        before = {str(p): p.read_bytes() for p in (self.root / "state").rglob("*") if p.is_file()}
        status, report = self.call(route)
        self.assertEqual(status, 200, report)
        self.assertEqual(report["completion_signal"], "DONE_TEST")
        self.assertEqual(report["progress"]["completed"], 1)
        self.assertEqual(report["progress"]["total"], 1)
        self.assertEqual(report, self.call(route)[1])
        after = {str(p): p.read_bytes() for p in (self.root / "state").rglob("*") if p.is_file()}
        self.assertEqual(before, after)
        self.assertEqual(self.call(route, headers={"Authorization": "Bearer wrong"})[0], 401)
        self.stop()
        self.start()
        self.assertEqual(report, self.call(route)[1])

    def test_work_report_failed_checks_have_no_completion_signal(self):
        r = self.eligible()
        self.stop()
        self.config["warrants"][r["id"]]["checks"] = [[sys.executable, "-c", "raise SystemExit(1)"]]
        self.start()
        status, run = self.call("/api/runs", "POST", {"warrant_id": r["id"], "source_sha256": r["source_sha256"]})
        self.assertEqual(status, 202)
        self.wait_run(run["attempt_id"])
        status, report = self.call("/api/runs/" + run["attempt_id"] + "/report")
        self.assertEqual(status, 200)
        self.assertIsNone(report["completion_signal"])
        self.assertEqual(report["progress"]["completed"], 0)

    def test_admission_is_read_only_and_start_rechecks(self):
        r = self.eligible()
        request = {"warrant_id": r["id"], "source_sha256": r["source_sha256"]}
        state = self.root / "state"
        def snapshot():
            return {str(p.relative_to(state)): p.read_bytes()
                    for p in state.rglob("*") if p.is_file()}
        before = snapshot()
        status, preview = self.call("/api/admission", "POST", request)
        self.assertEqual(status, 200, preview)
        self.assertEqual(preview["state"], "ready")
        self.assertEqual(preview["subject"], request)
        self.assertFalse(preview["dispatch_permitted"])
        self.assertFalse(preview["qualified"])
        self.assertEqual(before, snapshot())
        self.assertFalse((self.repo / ".git/openwarrant-execution").exists())
        status, run = self.call("/api/runs", "POST", request)
        self.assertEqual(status, 202, run)
        self.assertEqual(self.wait_run(run["attempt_id"])["work_state"], "completed")
        preview = self.call("/api/admission", "POST", request)[1]
        self.assertEqual(preview["state"], "blocked")
        self.assertEqual(preview["reason"], "Exact subject already completed")
        status, refusal = self.call("/api/runs", "POST", request)
        self.assertEqual(status, 409)
        self.assertEqual(refusal["error"], preview["reason"])

    def test_admission_refusals_match_start(self):
        r = self.eligible()
        request = {"warrant_id": r["id"], "source_sha256": r["source_sha256"]}
        cases = [
            ({**request, "source_sha256": "0" * 64}, "Changed subject; review execution configuration"),
            ({**request, "warrant_id": str(uuid.uuid4())}, "Warrant not configured for execution"),
        ]
        for fields, reason in cases:
            status, preview = self.call("/api/admission", "POST", fields)
            self.assertEqual(status, 200)
            self.assertEqual(preview["state"], "blocked")
            self.assertEqual(preview["reason"], reason)
            self.assertEqual(self.call("/api/runs", "POST", fields)[1]["error"], reason)
        self.assertEqual(self.call("/api/admission", "POST", {**request, "actor": "admin"})[0], 400)
        for verified, cost, expected in [
            (True, "free", "Verified-start requirement unsupported by this unverified workflow"),
            (False, "unknown", "Unknown cost cannot satisfy a hard spend cap"),
        ]:
            self.stop()
            self.config["warrants"][r["id"]]["verified_start"] = verified
            self.config["cost_mode"] = cost
            self.start()
            preview = self.call("/api/admission", "POST", request)[1]
            self.assertEqual(preview["state"], "blocked")
            self.assertEqual(preview["reason"], expected)
            self.assertEqual(self.call("/api/runs", "POST", request)[1]["error"], expected)
        self.assertEqual(self.call("/api/runs")[1]["runs"], [])

    def test_changed_dependency_checks_invalidate_completion_eligibility(self):
        dependency = self.eligible()
        status, run = self.call("/api/runs", "POST", {
            "warrant_id": dependency["id"], "source_sha256": dependency["source_sha256"]})
        self.assertEqual(status, 202, run)
        self.assertEqual(self.wait_run(run["attempt_id"])["work_state"], "completed")
        self.draft_id = str(uuid.uuid4())
        child = self.eligible()
        self.stop()
        self.config["warrants"][child["id"]]["dependencies"] = [dependency["id"]]
        self.start()
        request = {"warrant_id": child["id"], "source_sha256": child["source_sha256"]}
        self.assertEqual(self.call("/api/admission", "POST", request)[1]["state"], "ready")
        self.stop()
        self.config["warrants"][dependency["id"]]["checks"].append(
            [sys.executable, "-c", "raise SystemExit(1)"])
        self.start()
        preview = self.call("/api/admission", "POST", request)[1]
        self.assertEqual(preview["state"], "blocked", preview)
        status, refusal = self.call("/api/runs", "POST", request)
        self.assertEqual(status, 409, refusal)
        self.assertEqual(refusal["error"], preview["reason"])
        status, report = self.call("/api/runs/" + run["attempt_id"] + "/report")
        self.assertEqual(status, 200, report)
        self.assertEqual(report["progress"]["completed"], 0)
        self.assertIn(dependency["id"], report["progress"]["pending"])
        # The old result remains a truthful completed historical attempt.
        self.assertEqual(self.call("/api/runs/" + run["attempt_id"])[1]["work_state"], "completed")

    def test_admission_unavailable_base_is_unknown(self):
        r = self.eligible()
        self.stop()
        self.config["warrants"][r["id"]]["base_commit"] = "0" * 40
        self.start()
        status, preview = self.call("/api/admission", "POST", {
            "warrant_id": r["id"], "source_sha256": r["source_sha256"]})
        self.assertEqual(status, 200)
        self.assertEqual(preview["state"], "unknown")
        self.assertFalse(preview["dispatch_permitted"])
        self.assertEqual(self.call("/api/runs")[1]["runs"], [])

    def test_execute_exact_draft_in_isolated_worktree(self):
        record = self.eligible()
        status, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": record["id"], "source_sha256": record["source_sha256"]},
        )
        self.assertEqual(status, 202, run)
        result = self.wait_run(run["attempt_id"])
        self.assertEqual(result["work_state"], "completed", result)
        self.assertFalse(result["qualified"])
        self.assertEqual(result["execution_state"], "stopped")
        self.assertTrue(result["result_revision"])
        self.assertFalse((self.repo / "result.txt").exists())
        self.assertEqual(
            Path(result["worktree"], "result.txt").read_text(), "implemented\n"
        )
        self.stop()
        self.start()
        self.assertEqual(self.call("/api/runs/" + run["attempt_id"])[1], result)

    def test_changed_subject_and_authority_input_refuse(self):
        r = self.eligible()
        self.assertEqual(
            self.call(
                "/api/runs", "POST", {"warrant_id": r["id"], "source_sha256": "0" * 64}
            )[0],
            409,
        )
        self.assertEqual(
            self.call(
                "/api/runs",
                "POST",
                {
                    "warrant_id": r["id"],
                    "source_sha256": r["source_sha256"],
                    "actor": "human",
                },
            )[0],
            400,
        )
        self.assertEqual(self.call("/api/runs")[1]["runs"], [])

    def test_unknown_cost_and_verified_start_refuse(self):
        r = self.eligible()
        request = {"warrant_id": r["id"], "source_sha256": r["source_sha256"]}
        self.stop()
        self.config["cost_mode"] = "unknown"
        self.start()
        self.assertEqual(self.call("/api/runs", "POST", request)[0], 403)
        self.stop()
        self.config["spend_limit_usd"] = None
        self.config["warrants"][r["id"]]["verified_start"] = True
        self.start()
        self.assertEqual(self.call("/api/runs", "POST", request)[0], 403)
        self.assertEqual(self.call("/api/runs")[1]["runs"], [])
        self.stop()
        self.config["warrants"][r["id"]]["verified_start"] = False
        self.start()
        status, run = self.call("/api/runs", "POST", request)
        self.assertEqual(status, 202, run)
        self.assertIsNone(self.wait_run(run["attempt_id"])["cost_usd"])

    def test_failing_check_never_completes(self):
        r = self.eligible()
        self.stop()
        self.config["warrants"][r["id"]]["checks"] = [
            [sys.executable, "-c", "raise SystemExit(3)"]
        ]
        self.start()
        status, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        self.assertEqual(status, 202, run)
        result = self.wait_run(run["attempt_id"])
        self.assertEqual(result["work_state"], "failed")
        self.assertIsNone(result["result_revision"])
        self.assertIn("Required check failed", result["cause"])

    def test_duplicate_timeout_and_restart_block_replacement(self):
        r = self.eligible()
        request = {"warrant_id": r["id"], "source_sha256": r["source_sha256"]}
        self.harness.write_text("import time; time.sleep(20)")
        self.stop()
        self.config["timeout_seconds"] = 1
        self.start()
        status, run = self.call("/api/runs", "POST", request)
        self.assertEqual(status, 202, run)
        self.assertEqual(self.call("/api/runs", "POST", request)[0], 409)
        result = self.wait_run(run["attempt_id"])
        self.assertEqual(result["execution_state"], "unknown")
        self.assertEqual(result["work_state"], "blocked")
        self.stop()
        self.start()
        self.assertEqual(self.call("/api/runs", "POST", request)[0], 409)

    def test_nonreading_harness_respects_deadline(self):
        r = self.eligible()
        self.harness.write_text("import time; time.sleep(20)")
        self.stop()
        self.config["timeout_seconds"] = 1
        self.start()
        # Large source exceeds typical pipe capacity; runner must not block writing stdin.
        draft = {
            **self.draft(),
            "id": r["id"],
            "outcome": "x" * 15900,
            "scope": "y" * 15900,
            "context": "z" * 15900,
            "expected_source_sha256": r["source_sha256"],
        }
        status, r = self.call("/api/warrants/" + r["id"], "PUT", draft)
        self.assertEqual(status, 200, r)
        self.stop()
        self.config["warrants"][r["id"]]["source_sha256"] = r["source_sha256"]
        self.start()
        status, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        self.assertEqual(status, 202, run)
        self.assertEqual(self.wait_run(run["attempt_id"])["execution_state"], "unknown")

    def test_restart_during_live_attempt_is_unknown(self):
        import signal

        r = self.eligible()
        pid_file = self.root / "child.pid"
        self.harness.write_text(
            "import os,time,pathlib; pathlib.Path("
            + repr(str(pid_file))
            + ").write_text(str(os.getpid())); time.sleep(20)"
        )
        status, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        self.assertEqual(status, 202, run)
        for _ in range(100):
            if pid_file.exists():
                break
            time.sleep(0.02)
        self.assertTrue(pid_file.exists())
        try:
            self.stop()
            self.start()
            loaded = self.call("/api/runs/" + run["attempt_id"])[1]
            self.assertEqual(loaded["execution_state"], "unknown")
            self.assertEqual(loaded["work_state"], "blocked")
            self.assertEqual(
                self.call(
                    "/api/runs",
                    "POST",
                    {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
                )[0],
                409,
            )
        finally:
            try:
                os.killpg(int(pid_file.read_text()), signal.SIGKILL)
            except ProcessLookupError:
                pass

    def test_other_state_cannot_create_second_warrant_worktree(self):
        r = self.eligible()
        request = {"warrant_id": r["id"], "source_sha256": r["source_sha256"]}
        status, run = self.call("/api/runs", "POST", request)
        self.assertEqual(status, 202, run)
        self.wait_run(run["attempt_id"])
        self.stop()
        self.state_name = "other-state"
        self.start()
        self.call(
            "/api/warrants",
            "POST",
            {k: r[k] for k in ("id", "title", "outcome", "scope", "context")},
        )
        status, result = self.call("/api/runs", "POST", request)
        self.assertEqual(status, 409, result)
        self.assertIn("another execution store", result["error"])

    def test_explicit_retry_budget_and_result_identity(self):
        r = self.eligible()
        request = {"warrant_id": r["id"], "source_sha256": r["source_sha256"]}
        self.harness.write_text(
            "import json,sys; r=json.load(sys.stdin); print(json.dumps({'schema':'oh.war/execution-result/v1','attempt_id':r['attempt_id'],'source_sha256':r['source_sha256'],'work_state':'failed','notes':'Observed failure','next_steps':['Repair']}))"
        )
        for _ in range(4):
            status, run = self.call("/api/runs", "POST", request)
            self.assertEqual(status, 202, run)
            self.assertEqual(self.wait_run(run["attempt_id"])["work_state"], "failed")
        self.assertEqual(self.call("/api/runs", "POST", request)[0], 409)

    def test_forged_result_never_completes(self):
        r = self.eligible()
        self.harness.write_text(
            "import json; print(json.dumps({'schema':'oh.war/execution-result/v1','attempt_id':'fake','source_sha256':'fake','work_state':'completed','notes':'done','next_steps':[]}))"
        )
        status, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        self.assertEqual(status, 202, run)
        result = self.wait_run(run["attempt_id"])
        self.assertEqual(result["work_state"], "failed")
        self.assertIn("subject mismatch", result["cause"])

    def test_duplicate_result_fields_refuse(self):
        r = self.eligible()
        self.harness.write_text(
            "import json,sys; r=json.load(sys.stdin); d={'schema':'oh.war/execution-result/v1','attempt_id':r['attempt_id'],'source_sha256':r['source_sha256'],'work_state':'completed','notes':'done','next_steps':[]}; print('{\"work_state\":\"failed\",'+json.dumps(d)[1:])"
        )
        self.stop()
        self.config["warrants"][r["id"]]["checks"] = [[sys.executable, "-c", "pass"]]
        self.start()
        _, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        self.assertEqual(self.wait_run(run["attempt_id"])["work_state"], "failed")

    def test_timeout_retains_observed_diagnostics(self):
        r = self.eligible()
        self.harness.write_text(
            "import time,sys; print('before timeout',file=sys.stderr,flush=True); time.sleep(20)"
        )
        self.stop()
        self.config["timeout_seconds"] = 1
        self.start()
        _, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        result = self.wait_run(run["attempt_id"])
        self.assertIn("before timeout", result.get("stderr", ""))

    def test_unicode_check_output_cannot_lose_terminal_record(self):
        r = self.eligible()
        self.stop()
        self.config["warrants"][r["id"]]["checks"] = [
            [sys.executable, "-c", "print('😀'*8000)"] for _ in range(16)
        ]
        self.start()
        _, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        result = self.wait_run(run["attempt_id"])
        self.assertEqual(result["sequence"], 2, result)
        self.assertEqual(result["work_state"], "completed", result)
        self.assertEqual(len(result["checks"]), 16)

    def test_invalid_unicode_result_persists_failed_terminal_record(self):
        r = self.eligible()
        self.harness.write_text(
            "import json,sys; r=json.load(sys.stdin); print(json.dumps({'schema':'oh.war/execution-result/v1','attempt_id':r['attempt_id'],'source_sha256':r['source_sha256'],'work_state':'completed','notes':chr(0xd800),'next_steps':[]}))"
        )
        _, run = self.call(
            "/api/runs",
            "POST",
            {"warrant_id": r["id"], "source_sha256": r["source_sha256"]},
        )
        result = self.wait_run(run["attempt_id"])
        self.assertEqual(result["sequence"], 2, result)
        self.assertEqual(result["work_state"], "failed", result)

    def test_board_is_authenticated_read_only_and_matches_cli(self):
        # This execution fixture is a Git repo without an OpenWarrant corpus.
        # Preserve the refusal instead of fabricating an empty successful board.
        status, _ = self.call("/api/board", headers={"Authorization": "Bearer wrong"})
        self.assertEqual(status, 401)
        status, result = self.call("/api/board")
        self.assertEqual(status, 422, result)
        self.assertEqual(result["error"], "SDK refused input")
        self.assertFalse(result["qualified"])

    # Existing corpus-inventory test expects the real OpenWarrant repository.
    test_project_inventory_keeps_legacy_and_work_separate = None


if __name__ == "__main__":
    import unittest

    unittest.main()
