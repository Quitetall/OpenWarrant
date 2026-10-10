#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Transport/refusal controls with a synthetic server, never model qualification."""
import contextlib
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
                       "request": {"performer": "fixture-performer", "reviewed_subject": {
                           "contract_digest": "exact-before", "artifacts": {"file.rs": "before"}},
                           "obligations": [{"id": "OBL-001", "statement": "fixture"}]}}
        self.path = self.root / "bundle.json"
        self.path.write_text(json.dumps(self.bundle))
        self.received = []
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

    def run_client(self, base, packets=True, cwd=None):
        env = dict(os.environ, HTTP_PROXY="http://192.0.2.1:9", HTTPS_PROXY="http://192.0.2.1:9")
        env.pop("OPENWARRANT_REVIEWED_PACKETS", None)
        if packets:
            env["OPENWARRANT_REVIEWED_PACKETS"] = json.dumps([{"path": "packet.json", "digest": "packet-before"}])
        return subprocess.run([sys.executable, str(HERE / "local-verifier.py"), "--endpoint", base,
                               "--model", "fixture-model", "--log-root", str(self.root / "logs"),
                               "--timeout", "3", str(self.path)], env=env, cwd=cwd, text=True, capture_output=True)

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

    def test_redirect_is_refused_without_answer(self):
        self.redirect = True
        with self.server() as base:
            result = self.run_client(base)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("redirect", result.stderr)
        self.assertEqual(len(self.received), 1)

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
