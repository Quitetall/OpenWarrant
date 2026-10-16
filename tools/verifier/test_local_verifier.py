#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Transport/refusal controls with a synthetic server, never model qualification."""
import contextlib
import hashlib
import http.server
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("local_verifier", HERE / "local-verifier.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class Controls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bundle = {"schema": "oh.war/verification-bundle/v2", "warrant": "fixture",
                       "budget_tokens": 48000, "estimated_tokens": 100, "over_budget": False,
                       "request": {"performer": "fixture-performer", "reviewed_subject": {
                           "contract_digest": "exact-before", "artifacts": {"file.rs": "before"}},
                           "obligations": [{"id": "OBL-001", "statement": "fixture"}]}}
        self.path = self.root / "bundle.json"
        self.path.write_text(json.dumps(self.bundle))
        self.received = []
        self.health_requests = 0
        self.response = {"model": "fixture-model", "choices": [{"finish_reason": "stop", "message": {
            "content": json.dumps({"verdicts": [{"obligation": "OBL-001", "disposition": "established",
                                                 "evidence": "synthetic transport control only"}]})}}],
                         "usage": {"prompt_tokens": 11, "completion_tokens": 12}}
        self.redirect = False
        self.replace_input = False

    @contextlib.contextmanager
    def server(self):
        owner = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                owner.health_requests += 1
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b'{"service":"lamu","status":"ok"}')

            def do_POST(self):
                owner.received.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
                if owner.replace_input:
                    changed = json.loads(owner.path.read_text())
                    changed["request"]["reviewed_subject"]["contract_digest"] = "changed-after-read"
                    owner.path.write_text(json.dumps(changed))
                if owner.redirect:
                    self.send_response(307)
                    self.send_header("Location", "http://192.0.2.1:9/")
                    self.end_headers()
                else:
                    self.send_response(200)
                    self.end_headers()
                    self.wfile.write(json.dumps(owner.response).encode())

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield "http://127.0.0.1:" + str(server.server_port)
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def run_client(self, base, packets=True, cwd=None, packet_refs=None, view_refs=None, extra_args=()):
        env = dict(os.environ, HTTP_PROXY="http://192.0.2.1:9", HTTPS_PROXY="http://192.0.2.1:9")
        env.pop("OPENWARRANT_REVIEWED_PACKETS", None)
        env.pop("OPENWARRANT_REVIEWED_VIEWS", None)
        if packets:
            env["OPENWARRANT_REVIEWED_PACKETS"] = json.dumps(packet_refs if packet_refs is not None else [{"path": "packet.json", "digest": "packet-before"}])
        if view_refs is not None:
            env["OPENWARRANT_REVIEWED_VIEWS"] = json.dumps(view_refs)
        return subprocess.run([sys.executable, str(HERE / "local-verifier.py"), "--endpoint", base,
                               "--model", "fixture-model", "--log-root", str(self.root / "logs"),
                               "--timeout", "3", *extra_args, str(self.path)], env=env, cwd=cwd, text=True, capture_output=True, timeout=10)

    def test_operator_can_disable_thinking_without_changing_review_input(self):
        with self.server() as base:
            result = self.run_client(base, extra_args=("--thinking", "off"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIs(self.received[0]["enable_thinking"], False)
        self.assertEqual(json.loads(self.received[0]["messages"][1]["content"]), self.bundle)
        logs = list((self.root / "logs").glob("local-review-*"))
        run = json.loads((logs[0] / "run.json").read_text())
        self.assertEqual(run["thinking_requested"], "off")

    def test_operator_can_enable_thinking_and_invalid_mode_contacts_nobody(self):
        with self.server() as base:
            result = self.run_client(base, extra_args=("--thinking", "on"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIs(self.received[0]["enable_thinking"], True)
        self.received.clear()
        self.health_requests = 0
        before = self.path.read_bytes()
        existing_logs = set((self.root / "logs").iterdir())
        with self.server() as base:
            result = self.run_client(base, extra_args=("--thinking", "invalid"))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertEqual(self.health_requests, 0)
        self.assertEqual(self.received, [])
        self.assertEqual(set((self.root / "logs").iterdir()), existing_logs)
        self.assertEqual(self.path.read_bytes(), before)

    def test_disabled_thinking_does_not_admit_a_truncated_answer(self):
        self.response["choices"][0]["finish_reason"] = "length"
        with self.server() as base:
            result = self.run_client(base, extra_args=("--thinking", "off"))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("provider did not return one complete answer", result.stderr)
        self.assertIs(self.received[0]["enable_thinking"], False)
        logs = list((self.root / "logs").glob("local-review-*"))
        run = json.loads((logs[0] / "run.json").read_text())
        self.assertEqual(run["status"], "unavailable")
        self.assertEqual(run["thinking_requested"], "off")
        self.assertTrue((logs[0] / "provider-response.json").is_file())
        self.assertFalse((logs[0] / "response.toml").exists())

    def compact_input(self):
        repo=self.root/'repo';folder=repo/'docs/warrants/fixture/verifications';views=folder/'responses/views'
        views.mkdir(parents=True,exist_ok=True)
        archive=json.loads(json.dumps(self.bundle));archive['request']['schema']='oh.war/verification-request/v2'
        archive['over_budget']=True;archive['estimated_tokens']=90000
        raw=json.dumps(archive).encode();digest='a'*64;name='bundle-'+digest[:16]+'.json'
        (folder/name).write_bytes(raw);(views/name).write_bytes(raw)
        view=json.loads(json.dumps(self.bundle));view['schema']='oh.war/verification-view/v1'
        view['response_schema']='oh.war/verification-response/v3';view['dispatch_eligible']=True
        view['request'].pop('reviewed_subject');view['request']['source_schema']=archive['request']['schema']
        view['request']['reviewed_subject_archive']={'pointer':'/request/reviewed_subject'}
        view['archive']={'path':name,'digest':digest,'sha256':hashlib.sha256(raw).hexdigest(),'bytes':len(raw)}
        data=json.dumps(view).encode();sha=hashlib.sha256(data).hexdigest()
        self.path=views/('view-'+sha+'.json');self.path.write_bytes(data)
        packets=[{'path':(folder/name).relative_to(repo).as_posix(),'digest':digest}]
        refs=[{'path':self.path.relative_to(repo).as_posix(),'sha256':sha}]
        return repo,packets,refs,view

    def test_compact_view_is_the_only_model_input_and_response_binds_actual_pair(self):
        repo,packets,views,view=self.compact_input()
        with self.server() as base:
            result=self.run_client(base,cwd=repo,packet_refs=packets,view_refs=views)
        self.assertEqual(result.returncode,0,result.stderr)
        response=tomllib.loads(result.stdout)
        self.assertEqual(response['schema'],'oh.war/verification-response/v3')
        self.assertEqual(response['reviewed_packets'],packets)
        self.assertEqual(response['reviewed_views'],views)
        # TOML displays default-empty maps explicitly; the typed subject is
        # unchanged even when the source JSON omitted those optional maps.
        expected=self.bundle['request']['reviewed_subject']
        for key,value in expected.items():self.assertEqual(response['reviewed_subject'][key],value)
        for key,value in response['reviewed_subject'].items():
            if key not in expected:self.assertEqual(value,{})
        self.assertEqual(json.loads(self.received[0]['messages'][1]['content']),view)
        self.assertNotIn('reviewed_subject',json.loads(self.received[0]['messages'][1]['content'])['request'])

    def test_compact_binding_refusals_precede_provider_contact_and_log_writes(self):
        for kind in ['missing-view','wrong-view-hash','changed-sibling','changed-canonical','over-budget','not-eligible','preview-toggle','archive-link']:
            with self.subTest(kind=kind):
                repo,packets,views,view=self.compact_input()
                if kind=='missing-view':views=None
                elif kind=='wrong-view-hash':views[0]['sha256']='b'*64
                elif kind=='changed-sibling':(self.path.parent/view['archive']['path']).write_bytes(b'changed')
                elif kind=='changed-canonical':(repo/packets[0]['path']).write_bytes(b'changed')
                elif kind in ('over-budget','not-eligible','preview-toggle'):
                    if kind=='over-budget':view['over_budget']=True;view['estimated_tokens']=48001
                    elif kind=='not-eligible':view['dispatch_eligible']=False
                    else:view['schema']='oh.war/verification-view-preview/v1'
                    self.path.write_text(json.dumps(view))
                else:
                    sibling=self.path.parent/view['archive']['path'];outside=self.root/'outside-archive.json'
                    outside.write_bytes(sibling.read_bytes());sibling.unlink();sibling.symlink_to(outside)
                original=self.path.read_bytes()
                with self.server() as base:
                    result=self.run_client(base,cwd=repo,packet_refs=packets,view_refs=views)
                self.assertNotEqual(result.returncode,0,result.stderr)
                self.assertEqual(result.stdout,'')
                self.assertEqual(self.health_requests,0)
                self.assertEqual(self.received,[])
                self.assertFalse((self.root/'logs').exists())
                self.assertEqual(self.path.read_bytes(),original)

    def test_loopback_only_no_credentials_or_remote_routes(self):
        for bad in ("https://127.0.0.1:8020", "http://localhost:8020", "http://192.0.2.1:8020",
                    "http://127.0.0.1", "http://user:pass@127.0.0.1:8020", "http://127.0.0.1:8020/v1",
                    "http://127.0.0.1:8020?x=1", "http://127.0.0.1:8020#fragment"):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                verifier.endpoint(bad)
        self.assertEqual(verifier.endpoint("http://[::1]:8020/"), "http://[::1]:8020")

    def test_complete_answer_is_bound_to_immutable_input_and_recorded(self):
        self.replace_input = True
        with self.server() as base:
            result = self.run_client(base)
        self.assertEqual(result.returncode, 0, result.stderr)
        response = tomllib.loads(result.stdout)
        self.assertEqual(response["reviewed_subject"]["contract_digest"], "exact-before")
        self.assertEqual(response["reviewed_packets"], [{"path": "packet.json", "digest": "packet-before"}])
        sent = self.received[0]
        self.assertNotIn("enable_thinking", sent)
        self.assertEqual(len(sent["messages"]), 2)
        self.assertNotIn("tools", sent)
        self.assertEqual(json.loads(sent["messages"][1]["content"]), self.bundle)
        self.assertFalse(response["verifications"][0]["verifier"]["independence"]["distinct_model_required"])
        logs = list((self.root / "logs").glob("local-review-*"))
        self.assertEqual(len(logs), 1)
        self.assertEqual(json.loads((logs[0] / "bundle.json").read_text()), self.bundle)
        run = json.loads((logs[0] / "run.json").read_text())
        self.assertIsNone(run["api_charge_usd"])
        self.assertEqual(run["usage"]["completion_tokens"], 12)

    def test_compiler_budget_refusal_precedes_all_backend_calls_and_writes(self):
        cases = [
            {"budget_tokens": 48000, "estimated_tokens": 48001, "over_budget": False},
            {"budget_tokens": 48000, "estimated_tokens": 100, "over_budget": True},
            {"budget_tokens": True, "estimated_tokens": 100, "over_budget": False},
            {"budget_tokens": 48000, "estimated_tokens": -1, "over_budget": False},
            {"budget_tokens": 48000, "estimated_tokens": 2**64, "over_budget": False},
            {"budget_tokens": 48000, "estimated_tokens": 100, "over_budget": "false"},
            {"budget_tokens": None, "estimated_tokens": 100, "over_budget": False},
        ]
        for fields in cases:
            with self.subTest(fields=fields):
                self.bundle.update(fields)
                self.path.write_text(json.dumps(self.bundle))
                before = self.path.read_bytes()
                with self.server() as base:
                    result = self.run_client(base)
                self.assertNotEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "")
                self.assertIn("verify.bundle-", result.stderr)
                self.assertEqual(self.received, [])
                self.assertEqual(self.health_requests, 0)
                self.assertFalse((self.root / "logs").exists())
                self.assertEqual(self.path.read_bytes(), before)

    def test_redirect_is_refused_without_answer(self):
        self.redirect = True
        with self.server() as base:
            result = self.run_client(base)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("redirect", result.stderr)
        self.assertEqual(len(self.received), 1)

    def test_review_preview_is_not_an_executable_bundle(self):
        self.bundle["schema"] = "oh.war/verification-view-preview/v1"
        self.bundle["dispatch_eligible"] = False
        self.path.write_text(json.dumps(self.bundle))
        before = self.path.read_bytes()
        with self.server() as base:
            result = self.run_client(base)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("only a current v2 verification bundle", result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(self.health_requests, 0)
        self.assertEqual(self.received, [])
        self.assertFalse((self.root / "logs").exists())
        self.assertEqual(self.path.read_bytes(), before)

    def test_substituted_model_tool_call_and_truncation_are_refused(self):
        for kind in ("model", "tool", "truncated"):
            with self.subTest(kind=kind):
                if kind == "model":
                    self.response["model"] = "other-model"
                elif kind == "tool":
                    self.response["model"] = "fixture-model"
                    self.response["choices"][0]["message"]["tool_calls"] = [{"function": "write"}]
                else:
                    self.response["choices"][0]["message"].pop("tool_calls")
                    self.response["choices"][0]["finish_reason"] = "length"
                with self.server() as base:
                    result = self.run_client(base)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")

    def test_missing_packet_binding_never_calls_provider(self):
        with self.server() as base:
            result = self.run_client(base, packets=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.received, [])

    def test_logs_inside_checked_source_are_refused_before_writes_or_call(self):
        with self.server() as base:
            result = self.run_client(base, cwd=self.root)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("outside the repository", result.stderr)
        self.assertEqual(self.received, [])
        self.assertFalse((self.root / "logs").exists())

    def test_malformed_partial_duplicate_and_foreign_verdicts_do_not_establish(self):
        good = {"obligation": "OBL-001", "disposition": "established", "evidence": "synthetic"}
        for text in ("not json", '{"verdicts":[]}', '{"verdicts":[],"verdicts":[]}',
                     json.dumps({"verdicts": [good, good]}),
                     json.dumps({"verdicts": [dict(good, obligation="OTHER")]}),
                     json.dumps({"verdicts": [dict(good, disposition="probably")]})):
            with self.subTest(text=text):
                rows = verifier.verdicts(text, self.bundle["request"]["obligations"])
                self.assertEqual(rows["OBL-001"]["disposition"], "not_established")


if __name__ == "__main__":
    unittest.main()
