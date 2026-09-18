# SPDX-License-Identifier: Apache-2.0
"""Real HTTP/process/Git execution seam; synthetic harness, no model calls."""

import json
import hashlib
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

    def staged(self):
        draft = self.eligible()
        self.stop()
        self.config["schema"] = "oh.war/execution-config/v2"
        self.config["repair_cycles"] = 0
        self.config["warrants"][draft["id"]]["stage_plan"] = {
            "schema": "oh.war/execution-stage-plan/v1", "stages": {
                "api": {"title": "API", "outcome": "Write API", "dependencies": [],
                        "checks": [[sys.executable, "-c", "from pathlib import Path; assert Path('api.txt').read_text()=='api'"]]},
                "ui": {"title": "UI", "outcome": "Write UI", "dependencies": ["api"],
                       "checks": [[sys.executable, "-c", "from pathlib import Path; assert Path('ui.txt').read_text()=='ui'"]]}}}
        self.config["warrants"][draft["id"]]["checks"] = [[sys.executable, "-c",
            "from pathlib import Path; assert Path('api.txt').exists() and Path('ui.txt').exists()"]]
        self.harness.write_text("""import json,sys,pathlib,subprocess
r=json.load(sys.stdin)
assert r['schema']=='oh.war/execution-request/v3'
p=pathlib.Path(r['worktree']); stage=r['stage']
(p/(stage+'.txt')).write_text(stage)
subprocess.run(['git','add','.'],cwd=p,check=True)
subprocess.run(['git','commit','-qm',stage],cwd=p,check=True)
print(json.dumps({'schema':'oh.war/execution-result/v1','attempt_id':r['attempt_id'],
'source_sha256':r['source_sha256'],'work_state':'completed','notes':stage,'next_steps':[]}))
""")
        self.start()
        return {"warrant_id": draft["id"], "source_sha256": draft["source_sha256"]}

    def test_stages_dispatch_in_order_and_only_full_result_completes_warrant(self):
        fields = self.staged()
        self.assertEqual(self.call("/api/runs", "POST", fields)[0], 400)
        self.assertEqual(self.call("/api/runs", "POST", {**fields, "stage": "ui"})[0], 409)
        status, first = self.call("/api/runs", "POST", {**fields, "stage": "api"})
        self.assertEqual(status, 202, first)
        first = self.wait_run(first["attempt_id"])
        self.assertEqual(first["work_state"], "in-progress", first)
        self.assertEqual(first["execution_state"], "stopped")
        report = self.call("/api/runs/" + first["attempt_id"] + "/report")[1]
        self.assertIsNone(report["completion_signal"])
        self.assertEqual(report["progress"]["completed"], 0)
        self.assertEqual(self.call("/api/runs", "POST", {**fields, "stage": "api"})[0], 409)
        status, last = self.call("/api/runs", "POST", {**fields, "stage": "ui"})
        self.assertEqual(status, 202, last)
        last = self.wait_run(last["attempt_id"])
        self.assertEqual(last["work_state"], "completed", last)
        self.assertEqual(last["worktree"], first["worktree"])
        self.assertLess(last["remaining_seconds"], first["remaining_seconds"])
        report = self.call("/api/runs/" + last["attempt_id"] + "/report")[1]
        self.assertEqual(report["progress"]["completed"], 1)
        self.assertEqual(report["completion_signal"], "WORK_DONE")

    def test_stage_cannot_regress_previously_completed_prerequisite(self):
        fields = self.staged()
        first = self.call("/api/runs", "POST", {**fields, "stage": "api"})[1]
        self.assertEqual(self.wait_run(first["attempt_id"])["work_state"], "in-progress")
        self.harness.write_text(self.harness.read_text().replace(
            "(p/(stage+'.txt')).write_text(stage)",
            "(p/(stage+'.txt')).write_text(stage); (p/'api.txt').write_text('broken')"))
        status, last = self.call("/api/runs", "POST", {**fields, "stage": "ui"})
        self.assertEqual(status, 202, last)
        last = self.wait_run(last["attempt_id"])
        self.assertEqual(last["work_state"], "failed", last)
        self.assertEqual(last["execution_state"], "stopped")
        self.assertEqual(last["stage_checkpoint"]["skipped_stages"], {"ui": ["api"]})
        self.assertIsNone(self.call("/api/runs/" + last["attempt_id"] + "/report")[1]["completion_signal"])

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

    def test_hotline_checkpoint_waits_without_completion_or_start_bypass(self):
        draft = self.eligible()
        completion_harness = self.harness.read_text()
        self.stop(); self.config["repair_cycles"] = 0; self.start()
        self.harness.write_text("""import json,sys
r=json.load(sys.stdin)
print(json.dumps({'schema':'oh.war/execution-question/v1','attempt_id':r['attempt_id'],
'source_sha256':r['source_sha256'],'notes':'Existing base is a committed checkpoint.',
'next_steps':['Answer then resume'],'question':{'kind':'technical','text':'Which parser?',
'direct_human':False,'affected_stages':['STAGE-001']}}))
""")
        fields = {"warrant_id": draft["id"], "source_sha256": draft["source_sha256"]}
        status, run = self.call("/api/runs", "POST", fields)
        self.assertEqual(status, 202, run)
        stopped = self.wait_run(run["attempt_id"])
        self.assertEqual(stopped["execution_state"], "stopped", stopped)
        self.assertEqual(stopped["work_state"], "blocked")
        self.assertEqual(stopped["question"]["checkpoint"], self.base)
        self.assertIsNone(stopped["result_revision"])
        report = self.call("/api/runs/" + run["attempt_id"] + "/report")[1]
        self.assertIsNone(report["completion_signal"])
        self.assertEqual(self.call("/api/runs", "POST", fields)[0], 409)
        self.stop(); self.start()
        self.assertEqual(self.call("/api/runs/" + run["attempt_id"])[1], stopped)
        self.assertEqual(self.call("/api/admission", "POST", fields)[1]["state"], "blocked")

        self.assertEqual(self.call("/api/hotline")[0], 409)
        responder_token = "fixture-only-responder-credential-001"
        config = self.root / "hotline.json"
        config.write_text(json.dumps({"schema": "oh.war/hotline-config/v1", "responders": [{
            "id": "configured-adviser", "kind": "ai", "governing_warrants": [],
            "token_sha256": hashlib.sha256(responder_token.encode()).hexdigest()}]}))
        self.stop(); self.report_args = ["--hotline-config", str(config)]; self.start()
        status, listing = self.call("/api/hotline")
        self.assertEqual(status, 200, listing)
        self.assertEqual(len(listing["questions"]), 1)
        self.assertIsNone(listing["questions"][0]["answer"])
        route = "/api/hotline/" + run["attempt_id"] + "/answer"
        response = {"question_sha256": listing["questions"][0]["question_sha256"],
                    "answer": "Use the existing SDK parser.", "evidence": ["SDK parser API"]}
        resume_route = "/api/hotline/" + run["attempt_id"] + "/resume"
        resume_request = {"question_sha256": response["question_sha256"]}
        self.assertEqual(self.call(resume_route, "POST", resume_request)[0], 409)
        headers = {"X-OW-Responder": responder_token}
        self.assertEqual(self.call(route, "POST", response)[0], 401)
        self.assertEqual(self.call(route, "POST", response, {"X-OW-Responder": "wrong" * 10})[0], 401)
        self.assertEqual(self.call(route, "POST", {**response, "actor": "owner"}, headers)[0], 400)
        self.assertEqual(self.call(route, "POST", {**response, "question_sha256": "0" * 64}, headers)[0], 409)
        status, retained = self.call(route, "POST", response, headers)
        self.assertEqual(status, 200, retained)
        self.assertEqual(retained["respondent"], "configured-adviser")
        self.assertFalse(retained["qualified"])
        files = list((self.root / "state/.execution/hotline-answers").glob("*.json"))
        self.assertEqual(len(files), 1)
        original = files[0].read_bytes()
        self.assertEqual(self.call(route, "POST", response, headers)[1], retained)
        self.assertEqual(files[0].read_bytes(), original)
        self.assertEqual(self.call(route, "POST", {**response, "answer": "Different advice"}, headers)[0], 409)
        self.stop(); self.start()
        self.assertEqual(self.call("/api/hotline")[1]["questions"][0]["answer"], retained)
        self.assertEqual(self.call("/api/runs", "POST", fields)[0], 409)
        self.assertEqual(self.call(route, "POST", response, {**headers, "Authorization": "Bearer wrong"})[0], 401)
        original_config = config.read_bytes()
        self.stop()
        config.write_text(json.dumps({"schema": "oh.war/hotline-config/v1", "responders": []}))
        self.start()
        self.assertEqual(self.call(resume_route, "POST", resume_request)[0], 409)
        self.stop(); config.write_bytes(original_config); self.start()
        self.stop(); self.config["timeout_seconds"] = 6; self.start()
        self.assertEqual(self.call(resume_route, "POST", resume_request)[0], 409)
        self.stop(); self.config["timeout_seconds"] = 5; self.start()
        dirty = Path(stopped["worktree"]) / "unreviewed-change.txt"
        dirty.write_text("changed checkpoint")
        self.assertEqual(self.call(resume_route, "POST", resume_request)[0], 409)
        dirty.unlink()
        self.harness.write_text(completion_harness.replace("r=json.load(sys.stdin)",
            "r=json.load(sys.stdin)\nassert r['schema']=='oh.war/execution-request/v2'\n"
            "assert r['hotline_context'][0]['answer']['answer']=='Use the existing SDK parser.'\n"
            "assert 0 < r['limits']['timeout_seconds'] < 5"))
        status, resumed = self.call(resume_route, "POST", resume_request)
        self.assertEqual(status, 202, resumed)
        self.assertEqual(resumed["resume_from"], run["attempt_id"])
        finished = self.wait_run(resumed["attempt_id"])
        self.assertEqual(finished["work_state"], "completed", finished)
        self.assertEqual(self.call(resume_route, "POST", resume_request)[1], finished)
        self.assertEqual(len(self.call("/api/runs")[1]["runs"]), 2)
        self.assertIsNotNone(self.call("/api/runs/" + resumed["attempt_id"] + "/report")[1]["completion_signal"])

    def prepare_answered_question(self, delay=0, submit=True, kind="technical", direct_human=False):
        draft = self.eligible()
        completion = self.harness.read_text()
        credential = "public-hotline-fault-fixture-credential"
        config = self.root / "hotline-fault.json"
        config.write_text(json.dumps({"schema": "oh.war/hotline-config/v1", "responders": [{
            "id": "fixture-adviser", "kind": "ai", "governing_warrants": [],
            "token_sha256": hashlib.sha256(credential.encode()).hexdigest()}]}))
        self.stop(); self.config["timeout_seconds"] = 2
        self.report_args = ["--hotline-config", str(config)]; self.start()
        self.harness.write_text("import json,sys,time\nr=json.load(sys.stdin)\ntime.sleep(" + str(delay) + ")\n" +
            "print(json.dumps({'schema':'oh.war/execution-question/v1','attempt_id':r['attempt_id'],"
            "'source_sha256':r['source_sha256'],'notes':'Checkpoint retained','next_steps':[],"
            "'question':{'kind':'technical','text':'Which parser?','direct_human':False,'affected_stages':['STAGE-001']}}))")
        self.harness.write_text(self.harness.read_text().replace("'kind':'technical'", "'kind':" + repr(kind))
                                .replace("'direct_human':False", "'direct_human':" + repr(direct_human)))
        fields = {"warrant_id": draft["id"], "source_sha256": draft["source_sha256"]}
        status, run = self.call("/api/runs", "POST", fields)
        self.assertEqual(status, 202, run)
        stopped = self.wait_run(run["attempt_id"])
        self.assertEqual(stopped["execution_state"], "stopped", stopped)
        q = self.call("/api/hotline")[1]["questions"][0]
        answer = {"question_sha256": q["question_sha256"], "answer": "Use the SDK", "evidence": []}
        if submit:
            self.assertEqual(self.call("/api/hotline/" + run["attempt_id"] + "/answer", "POST", answer,
                                       {"X-OW-Responder": credential})[0], 200)
        return draft, stopped, completion, {"question_sha256": q["question_sha256"]}

    def enable_fixture_adviser(self, suffix="", cost_mode="free", cap=10):
        self.stop()
        script, marker = self.root / "adviser.py", self.root / "adviser-calls"
        script.write_text("import json,sys,pathlib,time\nr=json.load(sys.stdin)\n"
                         + "p=pathlib.Path(" + repr(str(marker)) + ");p.write_text(p.read_text()+'x' if p.exists() else 'x')\n"
                         + "assert r['scope'].startswith('technical advice only') and r['source']\n"
                         + (suffix or "print(json.dumps({'schema':'oh.war/hotline-advice/v1','question_sha256':r['question_sha256'],'answer':'Use the SDK parser','evidence':['SDK reference']}))"))
        config = self.root / "adviser.json"
        config.write_text(json.dumps({"schema": "oh.war/hotline-adviser/v1", "argv": [sys.executable, str(script)],
            "respondent": "fixture-adviser", "sandbox": "read-only-repository", "cost_mode": cost_mode,
            "spend_limit_usd": cap, "timeout_seconds": 1}))
        self.report_args += ["--adviser-config", str(config)]
        self.start()
        return marker

    def wait_advice(self):
        for _ in range(200):
            status, data = self.call("/api/hotline")
            self.assertEqual(status, 200, data)
            if data["advice"] and data["advice"][0]["state"] != "running":
                return data
            time.sleep(0.02)
        self.fail("adviser did not terminate")

    def test_automatic_advice_is_retained_without_resume_or_restart_duplicate(self):
        self.prepare_answered_question(submit=False)
        marker = self.enable_fixture_adviser()
        data = self.wait_advice()
        self.assertEqual(data["advice"][0]["state"], "answered", data)
        self.assertEqual(data["questions"][0]["answer"]["respondent"], "fixture-adviser")
        self.assertFalse(data["questions"][0]["answer"]["qualified"])
        self.assertEqual(len(self.call("/api/runs")[1]["runs"]), 1)
        self.stop(); self.start()
        self.assertEqual(self.call("/api/hotline")[1], data)
        self.assertEqual(marker.read_text(), "x")

        status, draft = self.call("/api/warrants", "POST", {**self.draft(), "id": str(uuid.uuid4())})
        self.assertEqual(status, 201, draft)
        self.config["warrants"][draft["id"]] = {
            **next(iter(self.config["warrants"].values())), "source_sha256": draft["source_sha256"]}
        self.stop(); self.start()
        status, run = self.call("/api/runs", "POST", {"warrant_id": draft["id"], "source_sha256": draft["source_sha256"]})
        self.assertEqual(status, 202, run)
        self.wait_run(run["attempt_id"])
        for _ in range(200):
            observed = self.call("/api/hotline")[1]["advice"]
            if len(observed) == 2 and all(r["state"] == "answered" for r in observed):
                break
            time.sleep(0.02)
        self.assertEqual(len(observed), 2)
        self.assertTrue(all(r["state"] == "answered" for r in observed), observed)
        self.assertEqual(marker.read_text(), "xx")

    def test_advice_unknown_cost_refuses_before_process_launch(self):
        self.prepare_answered_question(submit=False)
        marker = self.enable_fixture_adviser(cost_mode="unknown")
        data = self.wait_advice()
        self.assertEqual(data["advice"][0]["state"], "failed")
        self.assertIn("hard cap", data["advice"][0]["error"])
        self.assertIsNone(data["advice"][0]["cost_usd"])
        self.assertIsNone(data["questions"][0]["answer"])
        self.assertFalse(marker.exists())

    def test_advice_timeout_remains_unknown_without_restart_retry(self):
        self.prepare_answered_question(submit=False)
        marker = self.enable_fixture_adviser("time.sleep(20)")
        data = self.wait_advice()
        self.assertEqual(data["advice"][0]["state"], "unknown")
        self.assertIsNone(data["questions"][0]["answer"])
        self.stop(); self.start()
        self.assertEqual(self.call("/api/hotline")[1], data)
        self.assertEqual(marker.read_text(), "x")

    def test_adviser_cannot_inject_authority_fields(self):
        self.prepare_answered_question(submit=False)
        marker = self.enable_fixture_adviser("print(json.dumps({'schema':'oh.war/hotline-advice/v1','question_sha256':r['question_sha256'],'answer':'Invented permission','evidence':[],'qualified':True}))")
        data = self.wait_advice()
        self.assertEqual(data["advice"][0]["state"], "failed")
        self.assertIsNone(data["questions"][0]["answer"])
        self.stop(); self.start()
        self.assertEqual(marker.read_text(), "x")

    def test_interrupted_adviser_keeps_initial_record_unknown(self):
        import signal
        self.prepare_answered_question(submit=False)
        pidfile = self.root / "owned-adviser.pid"
        marker = self.enable_fixture_adviser(
            "import os;pathlib.Path(" + repr(str(pidfile)) + ").write_text(str(os.getpid()));time.sleep(20)")
        for _ in range(100):
            if pidfile.exists():
                break
            time.sleep(0.01)
        self.assertTrue(pidfile.exists())
        pid = int(pidfile.read_text())
        self.stop()
        try:
            self.start()
            data = self.call("/api/hotline")[1]
            self.assertEqual(data["advice"][0]["state"], "unknown")
            self.assertEqual(data["advice"][0]["sequence"], 1)
            self.assertIsNone(data["questions"][0]["answer"])
            self.assertEqual(marker.read_text(), "x")
        finally:
            # Only the synthetic process launched by this test, never a shared service.
            try:
                os.killpg(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    def test_governing_question_is_never_sent_to_automatic_adviser(self):
        self.prepare_answered_question(submit=False, kind="governing")
        marker = self.enable_fixture_adviser()
        self.assertEqual(self.call("/api/hotline")[1]["advice"], [])
        self.assertFalse(marker.exists())

    def test_direct_human_question_is_never_sent_to_automatic_adviser(self):
        self.prepare_answered_question(submit=False, direct_human=True)
        marker = self.enable_fixture_adviser()
        self.assertEqual(self.call("/api/hotline")[1]["advice"], [])
        self.assertFalse(marker.exists())

    def test_resume_preserves_consumed_time_and_unknown_refuses_new_writer(self):
        draft, prior, completion, request = self.prepare_answered_question(delay=0.8)
        self.harness.write_text(completion.replace("r=json.load(sys.stdin)",
                               "r=json.load(sys.stdin)\nimport time;time.sleep(1.6)"))
        route = "/api/hotline/" + prior["attempt_id"] + "/resume"
        status, run = self.call(route, "POST", request)
        self.assertEqual(status, 202, run)
        self.assertLess(run["remaining_seconds"], 1.6)
        final = self.wait_run(run["attempt_id"])
        self.assertEqual(final["execution_state"], "unknown", final)
        self.assertNotEqual(final["work_state"], "completed")
        self.assertIsNone(self.call("/api/runs/" + run["attempt_id"] + "/report")[1]["completion_signal"])
        self.assertEqual(self.call(route, "POST", request)[1], final)
        self.assertEqual(len(self.call("/api/runs")[1]["runs"]), 2)
        fields = {"warrant_id": draft["id"], "source_sha256": draft["source_sha256"]}
        self.assertEqual(self.call("/api/runs", "POST", fields)[0], 409)

    def test_concurrent_resume_claims_launch_one_attempt(self):
        from concurrent.futures import ThreadPoolExecutor
        _, prior, completion, request = self.prepare_answered_question()
        self.harness.write_text(completion)
        route = "/api/hotline/" + prior["attempt_id"] + "/resume"
        with ThreadPoolExecutor(max_workers=2) as pool:
            replies = list(pool.map(lambda _: self.call(route, "POST", request), range(2)))
        self.assertTrue(all(status == 202 for status, _ in replies), replies)
        self.assertEqual(replies[0][1]["attempt_id"], replies[1][1]["attempt_id"])
        self.assertEqual(self.wait_run(replies[0][1]["attempt_id"])["work_state"], "completed")
        self.assertEqual(len(self.call("/api/runs")[1]["runs"]), 2)

    def test_hotline_blocks_dependency_but_not_independent_warrant(self):
        parent, _, completion, _ = self.prepare_answered_question()
        self.harness.write_text(completion)
        subjects = []
        for dependent in (False, True):
            status, draft = self.call("/api/warrants", "POST", {**self.draft(), "id": str(uuid.uuid4())})
            self.assertEqual(status, 201, draft)
            self.config["warrants"][draft["id"]] = {
                **self.config["warrants"][parent["id"]], "source_sha256": draft["source_sha256"],
                "dependencies": [parent["id"]] if dependent else []}
            subjects.append({"warrant_id": draft["id"], "source_sha256": draft["source_sha256"]})
        self.stop(); self.start()
        self.assertEqual(self.call("/api/runs", "POST", subjects[1])[0], 409)
        status, independent = self.call("/api/runs", "POST", subjects[0])
        self.assertEqual(status, 202, independent)
        self.assertEqual(self.wait_run(independent["attempt_id"])["work_state"], "completed")
        self.assertEqual(self.call("/api/admission", "POST", subjects[1])[1]["state"], "blocked")

    def test_editing_question_source_refuses_resume(self):
        draft, prior, _, request = self.prepare_answered_question()
        changed = {**self.draft(), "id": draft["id"], "outcome": "A different required outcome",
                   "expected_source_sha256": draft["source_sha256"]}
        self.assertEqual(self.call("/api/warrants/" + draft["id"], "PUT", changed)[0], 200)
        self.assertEqual(self.call("/api/hotline/" + prior["attempt_id"] + "/resume", "POST", request)[0], 409)
        self.assertEqual(len(self.call("/api/runs")[1]["runs"]), 1)

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
