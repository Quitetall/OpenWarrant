# SPDX-License-Identifier: Apache-2.0
"""Actual HTTP/SDK/Git recovery; synthetic workers/issuer, never qualification."""
import base64
import hashlib
import json
import random
import subprocess
import sys
import time
import unittest
import uuid
from pathlib import Path

import test_execution as execution_tests
from test_workflow import WAR
from verifier_attestation import NAMESPACE
from verifier_policy import PROTECTIONS


class RuntimeRecoveryTests(unittest.TestCase):
    def setUp(self):
        class ObservedFixture(execution_tests.ExecutionTests):
            def stop(self):
                self.proc.terminate()
                self.proc.wait(timeout=10)
                diagnostic = self.proc.stderr.read().decode(errors="replace")
                self.proc.stderr.close()
                if diagnostic:
                    sys.stderr.write("Recovery fixture service stderr:\n" + diagnostic[-16000:] + "\n")

        self.fixture = t = ObservedFixture()
        t.setUp()
        self.addCleanup(t.tearDown)
        self.fields = t.linked_runtime()
        t.stop()
        graph = t.repo / "docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml"
        text = graph.read_text().replace('executor_kind: "agent"',
            'executor_kind: "katana"\n    executor_ref: "synthetic"')
        for stage, path in (("STAGE-001", "build-context.md"), ("STAGE-002", "finish-context.md")):
            text = text.replace('  - id: "' + stage + '"\n',
                '  - id: "' + stage + '"\n    context_artifacts: ["' + path + '"]\n')
            (t.repo / path).write_text('---\nclassification: internal\n---\nInitial ' + stage + '.\n')
        graph.write_text(text)
        t.git("add", ".")
        t.git("commit", "-qm", "runtime recovery context fixture")
        t.base = t.git("rev-parse", "HEAD").strip()
        packet = t.root / "recovery-reference.json"
        subprocess.run([WAR, "--root", str(t.repo), "dispatch", "IX-WAR-0003", "STAGE-001",
                        "--prototype", "--emit", str(packet), "--json"],
                       capture_output=True, timeout=30, check=True)
        policy = t.config["warrants"][self.fields["warrant_id"]]
        policy["base_commit"] = t.base
        policy["runtime_link"]["contract_digest"] = json.loads(packet.read_bytes())["contract_digest"]
        policy["stage_plan"]["stages"]["finish"] = {
            "title": "Finish", "outcome": "Commit candidate", "dependencies": ["build"],
            "checks": policy["checks"]}
        policy["runtime_link"]["stages"]["finish"] = "STAGE-002"
        t.harness.write_text("""import base64,gzip,json,pathlib,subprocess,sys,tempfile
r=json.load(sys.stdin); p=pathlib.Path(r['worktree'])
def read_context(c,path):
 raw=gzip.decompress(base64.b64decode(c['bundle_base64'],validate=True))
 with tempfile.TemporaryDirectory() as name:
  f=pathlib.Path(name)/'bundle.json'; f.write_bytes(raw)
  out=subprocess.run([sys.argv[1],'dispatch-bundle','check',str(f),'--expected-digest',c['bundle_digest'],
                      '--read',path,'--json'],capture_output=True,check=True,timeout=10)
  checked=json.loads(out.stdout)['result']; assert checked['execution_authorized'] is False
  return checked['content']
def write_context(path,text):
 (p/path).write_text('---\\nclassification: internal\\n---\\n'+text+'\\n')
if r.get('verification_repair'):
 assert r['schema']=='oh.war/execution-request/v6-draft.1'
 assert set(r['runtime_contexts'])=={'build','finish'}
 assert 'Build complete.' in read_context(r['runtime_contexts']['build'],'build-context.md')
 assert 'Candidate checkpoint.' in read_context(r['runtime_contexts']['finish'],'finish-context.md')
 assert r['runtime_contexts']['build']['stage_id']=='STAGE-001'
 assert r['runtime_contexts']['finish']['stage_id']=='STAGE-002'
 (p/'fixed.txt').write_text('fixed')
 write_context('finish-context.md','Repaired checkpoint.')
elif r.get('resume_from'):
 assert 'Question checkpoint.' in read_context(r['runtime_context'],'build-context.md')
 assert r['hotline_context'][0]['answer']['answer']=='Use the existing API.'
 write_context('build-context.md','Build complete.')
elif r['stage']=='build':
 assert 'Initial STAGE-001.' in read_context(r['runtime_context'],'build-context.md')
 write_context('build-context.md','Build complete.')
else:
 assert 'Initial STAGE-002.' in read_context(r['runtime_context'],'finish-context.md')
 write_context('finish-context.md','Candidate checkpoint.')
(p/'result.txt').write_text('implemented\\n')
subprocess.run(['git','add','.'],cwd=p,check=True,stdout=sys.stderr,timeout=10)
subprocess.run(['git','commit','-qm','recovery checkpoint'],cwd=p,check=True,stdout=sys.stderr,timeout=10)
print(json.dumps({'schema':'oh.war/execution-result/v1','attempt_id':r['attempt_id'],
'source_sha256':r['source_sha256'],'work_state':'completed','notes':'Synthetic checkpoint observed.',
'next_steps':[]}))
""")
        t.start()

    def wait_attempt(self, id):
        # Context compilation is part of the declared 30-second execution budget.
        # Observe its terminal record, not an unrelated six-second latency target.
        t = self.fixture
        deadline = time.monotonic() + t.config["timeout_seconds"] + 10
        while time.monotonic() < deadline:
            code, record = t.call("/api/runs/" + id)
            self.assertEqual(code, 200, record)
            if record["execution_state"] != "running":
                return record
            time.sleep(.02)
        self.fail("No terminal attempt observed within configured deadline: " + repr(record))

    def run_stage(self, stage):
        t = self.fixture
        code, run = t.call("/api/runs", "POST", {**self.fields, "stage": stage})
        self.assertEqual(code, 202, run)
        run = self.wait_attempt(run["attempt_id"])
        self.assertEqual(run["work_state"], "in-progress" if stage == "build" else "completed", run)
        return run

    def test_hotline_resume_receives_committed_question_checkpoint_context(self):
        t = self.fixture
        implementation = t.harness.read_text()
        t.stop()
        token = "fixture-only-recovery-adviser-credential"
        hotline = t.root / "hotline.json"
        hotline.write_text(json.dumps({"schema": "oh.war/hotline-config/v1", "responders": [{
            "id": "synthetic-adviser", "kind": "ai", "governing_warrants": [],
            "token_sha256": hashlib.sha256(token.encode()).hexdigest()}]}))
        t.report_args = ["--hotline-config", str(hotline)]
        t.harness.write_text("""import json,pathlib,subprocess,sys
r=json.load(sys.stdin); p=pathlib.Path(r['worktree'])
(p/'build-context.md').write_text('---\\nclassification: internal\\n---\\nQuestion checkpoint.\\n')
subprocess.run(['git','add','build-context.md'],cwd=p,check=True,timeout=10)
subprocess.run(['git','commit','-qm','question checkpoint'],cwd=p,check=True,timeout=10)
print(json.dumps({'schema':'oh.war/execution-question/v1','attempt_id':r['attempt_id'],
'source_sha256':r['source_sha256'],'notes':'Committed partial work.','next_steps':['Answer and resume'],
'question':{'kind':'technical','text':'Use existing API?','direct_human':False,'affected_stages':['build']}}))
""")
        t.start()
        code, paused = t.call("/api/runs", "POST", self.fields)
        self.assertEqual(code, 202, paused)
        paused = self.wait_attempt(paused["attempt_id"])
        self.assertEqual(paused["work_state"], "blocked", paused)
        self.assertNotEqual(paused["question"]["checkpoint"], t.base)
        question_hash = t.call("/api/hotline")[1]["questions"][0]["question_sha256"]
        route = "/api/hotline/" + paused["attempt_id"]
        self.assertEqual(t.call(route + "/resume", "POST", {"question_sha256": question_hash})[0], 409)
        code, answer = t.call(route + "/answer", "POST", {"question_sha256": question_hash,
            "answer": "Use the existing API.", "evidence": ["synthetic fixture contract"]},
            {"X-OW-Responder": token})
        self.assertEqual(code, 200, answer)
        t.harness.write_text(implementation)
        code, resumed = t.call(route + "/resume", "POST", {"question_sha256": question_hash})
        self.assertEqual(code, 202, resumed)
        resumed = self.wait_attempt(resumed["attempt_id"])
        self.assertEqual(resumed["work_state"], "in-progress", resumed)
        self.assertEqual(resumed["resume_from"], paused["attempt_id"])
        self.assertEqual(resumed["input_revision"], paused["question"]["checkpoint"])
        self.assertNotEqual(resumed["runtime_dispatch"]["dispatch_id"], paused["runtime_dispatch"]["dispatch_id"])
        self.assertEqual(t.git("rev-parse", "HEAD").strip(), t.base)
        self.assertFalse(self.run_stage("finish")["qualified"])

    def install_verifier(self):
        t = self.fixture
        t.stop()
        self.key = t.root / "synthetic-machine-key"
        subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(self.key)],
                       capture_output=True, check=True, timeout=10)
        issuer = t.root / "issuer.json"
        issuer.write_text(json.dumps({"schema": "oh.war/verifier-issuer/v1", "principal": "fixture",
            "public_key": " ".join(self.key.with_suffix(".pub").read_text().split()[:2])}))
        reviewer = t.root / "reviewer.py"
        reviewer.write_text("""import hashlib,json,pathlib,sys
r=json.load(sys.stdin); ok=pathlib.Path('fixed.txt').exists()
h=hashlib.sha256(json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
print(json.dumps({'schema':'oh.war/verification-result/v1','verification_id':r['verification_id'],
'request_sha256':h,'verdict':'pass' if ok else 'fail','summary':'Synthetic marker check',
'findings':[] if ok else [{'id':'F1','observation':'Marker absent','scope':'fixed.txt fixture',
'evidence':['fixed.txt missing'],'status':'violation','repairable':True}]}))
""")
        config = t.root / "verifier.json"
        config.write_text(json.dumps({"schema": "oh.war/verifier-config/v1",
            "performer": {"id": "synthetic-worker", "argv": t.config["argv"]},
            "verifier": {"id": "synthetic-reviewer", "argv": [sys.executable, str(reviewer)]},
            "cost_mode": "free", "spend_limit_usd": 10, "timeout_seconds": 10}))
        t.report_args = ["--verifier-config", str(config), "--verifier-issuer", str(issuer)]
        t.start()

    def verify(self, attempt):
        t = self.fixture
        id = str(uuid.uuid4())
        code, prepared = t.call("/api/verification", "POST", {"attempt_id": attempt, "verification_id": id})
        self.assertEqual(code, 200, prepared)
        now = int(time.time())
        payload = json.dumps({"schema": "oh.war/harness-protection/v1", "basis_sha256": prepared["basis_sha256"],
            "nonce": id, "issued_at_unix": now, "expires_at_unix": now + 120,
            "evidence_ref": "fixture://synthetic-runtime-recovery",
            "protections": {p: "pass" for p in PROTECTIONS}}).encode()
        signature = subprocess.run(["ssh-keygen", "-Y", "sign", "-f", str(self.key), "-n", NAMESPACE],
            input=payload, capture_output=True, check=True, timeout=10).stdout
        route = "/api/verification/" + id
        code, started = t.call(route + "/start", "POST", {"payload_base64": base64.b64encode(payload).decode(),
            "signature_base64": base64.b64encode(signature).decode()})
        self.assertEqual(code, 202, started)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            code, job = t.call(route)
            self.assertEqual(code, 200, job)
            if job["state"] == "finished":
                return job
            time.sleep(.02)
        self.fail("Synthetic verification did not finish")

    def test_verifier_repair_receives_all_stage_contexts_from_candidate_checkpoint(self):
        t = self.fixture
        self.install_verifier()
        self.run_stage("build")
        original = self.run_stage("finish")
        failed = self.verify(original["attempt_id"])
        self.assertEqual(failed["effective_verdict"], "fail", failed)
        route = "/api/verification/" + failed["verification_id"] + "/repair"
        code, repair = t.call(route, "POST", {})
        self.assertEqual(code, 202, repair)
        repair = self.wait_attempt(repair["attempt_id"])
        self.assertEqual(repair["work_state"], "completed", repair)
        self.assertNotIn("stage", repair)
        self.assertEqual(set(repair["runtime_dispatches"]), {"build", "finish"})
        for stage in ("build", "finish"):
            self.assertNotEqual(repair["runtime_dispatches"][stage]["dispatch_id"],
                original["runtime_dispatch"]["dispatch_id"])
        self.assertEqual(repair["input_revision"], original["result_revision"])
        self.assertEqual(t.git("rev-parse", "HEAD").strip(), t.base)
        self.assertEqual(t.call(route, "POST", {})[1]["attempt_id"], repair["attempt_id"])
        passed = self.verify(repair["attempt_id"])
        self.assertEqual(passed["effective_verdict"], "pass", passed)
        self.assertFalse(passed["qualified"])
        self.assertFalse(repair["qualified"])

        def capture(binding):
            raw = b"synthetic unsigned repair-context receipt"
            return {"schema": "oh.war/runtime-capture-inline/v1-draft.1",
                "receipt_base64": base64.b64encode(raw).decode(),
                "request": {"schema": "oh.war/runtime-capture-request/v1-draft.1",
                    "dispatch_id": binding["dispatch_id"],
                    "receipt": "receipt://sha256:" + hashlib.sha256(raw).hexdigest(),
                    "provider": {"kind": "katana", "identity": "synthetic", "version": "fixture/v1"},
                    "metadata": {"observation_id": "repair-context-fixture", "observed_at": "2026-10-11T00:00:00Z",
                        "original_receipt_ref": "provider://synthetic/repair-context", "binary_identity": None,
                        "source_identity": None, "transport": "http-fixture", "argv": [], "exit_code": 0,
                        "status": "synthetic"}}}

        prior_capture = capture(original["runtime_dispatch"])
        code, prior = t.call("/api/runs/" + original["attempt_id"] + "/captures", "POST", {"capture": prior_capture})
        self.assertEqual(code, 200, prior)
        self.assertTrue(prior["capture"]["retained"])
        captures_route = "/api/runs/" + repair["attempt_id"] + "/captures"
        code, refused = t.call(captures_route, "POST", {"capture": prior_capture})
        self.assertEqual(code, 409, refused)
        self.assertIn("this attempt's retained Dispatch", refused["error"])
        self.assertEqual(t.call(captures_route)[1]["captures"], [])
        for binding in repair["runtime_dispatches"].values():
            code, attached = t.call(captures_route, "POST", {"capture": capture(binding)})
            self.assertEqual(code, 200, attached)
            self.assertEqual(attached["attachment"]["runtime_dispatch"], binding)
            self.assertFalse(attached["qualified"])
        recorded = t.call(captures_route)[1]
        self.assertEqual(len(recorded["captures"]), 2)
        t.stop()
        t.start()
        self.assertEqual(t.call(captures_route)[1], recorded)

    def test_combined_repair_context_refuses_before_worker_launch(self):
        t = self.fixture
        t.stop()
        graph = t.repo / "docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml"
        graph.write_text(graph.read_text().replace("    context_artifacts:",
            "    budget_tokens: 1000000\n    context_artifacts:"))
        for seed, path in enumerate(("build-context.md", "finish-context.md")):
            # Reproducible public noise: each package fits the transport limit,
            # while both required stage packages exceed the combined allowance.
            padding = base64.b64encode(random.Random(seed).randbytes(350000)).decode()
            original = (t.repo / path).read_text()
            (t.repo / path).write_text(original + "<!-- fixture padding -->" + padding)
        t.git("add", ".")
        t.git("commit", "-qm", "bounded combined transport fixture")
        t.base = t.git("rev-parse", "HEAD").strip()
        packet = t.root / "large-reference.json"
        subprocess.run([WAR, "--root", str(t.repo), "dispatch", "IX-WAR-0003", "STAGE-001",
                        "--prototype", "--emit", str(packet), "--json"],
                       capture_output=True, timeout=30, check=True)
        policy = t.config["warrants"][self.fields["warrant_id"]]
        policy["base_commit"] = t.base
        policy["runtime_link"]["contract_digest"] = json.loads(packet.read_bytes())["contract_digest"]
        program = t.harness.read_text().replace(
            " (p/path).write_text('---\\nclassification: internal\\n---\\n'+text+'\\n')",
            " padding=(p/path).read_text().split('<!-- fixture padding -->',1)[1]\n"
            " (p/path).write_text('---\\nclassification: internal\\n---\\n'+text+'\\n<!-- fixture padding -->'+padding)")
        self.assertIn("padding=(p/path)", program)
        program = program.replace("if r.get('verification_repair'):\n",
            "if r.get('verification_repair'):\n (p/'repair-launched.txt').write_text('launched')\n", 1)
        t.harness.write_text(program)
        t.start()
        self.install_verifier()
        self.run_stage("build")
        original = self.run_stage("finish")
        failed = self.verify(original["attempt_id"])
        self.assertEqual(failed["effective_verdict"], "fail", failed)
        code, repair = t.call("/api/verification/" + failed["verification_id"] + "/repair", "POST", {})
        self.assertEqual(code, 202, repair)
        repair = self.wait_attempt(repair["attempt_id"])
        self.assertEqual(repair["execution_state"], "stopped", repair)
        self.assertEqual(repair["work_state"], "failed", repair)
        self.assertEqual(repair["cause"], "Combined repair context exceeds the 1 MiB workflow transport limit")
        self.assertFalse((Path(repair["worktree"]) / "repair-launched.txt").exists())
        self.assertFalse(repair["qualified"])


if __name__ == "__main__":
    unittest.main()
