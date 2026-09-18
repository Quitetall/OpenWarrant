# SPDX-License-Identifier: Apache-2.0
import copy
import http.server
import json
import subprocess
import sys
import threading
import unittest
from pathlib import Path

from local_drafter import ATOMS, MILESTONES, check_schema, endpoint_url, loads, proposal_schema


def proposal():
    return {"api_version": "oh.war/draft-proposal/v2",
            "proposed_identity": {"title": "Synthetic adapter fixture", "profile": "delivery", "assurance": "basic"},
            "operations": [{"op": "create_atom", "role": role, "ordinal": ordinal, "path": path, "body": MILESTONES if role == "milestones" else "Synthetic fixture, not agent evidence"} for role, ordinal, path in ATOMS],
            "risk_assessment": "Synthetic transport control only"}


class AdapterTests(unittest.TestCase):
    def test_exact_fields_refuse_the_observed_path_error(self):
        check_schema(proposal(), proposal_schema())
        for field, value in (("path", "intent/10/10-intent.md"), ("ordinal", 1), ("ordinal", True), ("op", "write_file")):
            candidate = proposal()
            candidate["operations"][0][field] = value
            with self.assertRaises(ValueError):
                check_schema(candidate, proposal_schema())

    def test_missing_extra_and_overlong_fields_refuse(self):
        for mutation in (lambda p: p.update(approved=True), lambda p: p["operations"].pop(),
                         lambda p: p["operations"][0].update(body="x" * 1601)):
            candidate = proposal()
            mutation(candidate)
            with self.assertRaises(ValueError):
                check_schema(candidate, proposal_schema())
        candidate = proposal()
        candidate["operations"][3]["body"] = "oh.war/milestones/v1:\n  milestones: [M1]"
        with self.assertRaises(ValueError):
            check_schema(candidate, proposal_schema())
        with self.assertRaises(ValueError):
            loads('{"x":1,"x":2}')
        with self.assertRaises(ValueError):
            loads('{"x":NaN}')

    def test_endpoint_refuses_remote_credentials_and_paths(self):
        self.assertEqual(endpoint_url("http://127.0.0.1:1234"), "http://127.0.0.1:1234/v1/chat/completions")
        for url in ("https://example.com", "http://localhost:1234", "http://user:secret@127.0.0.1", "http://127.0.0.1/path", "http://127.0.0.1?token=x"):
            with self.assertRaises(ValueError):
                endpoint_url(url)

    def test_public_process_emits_only_complete_conforming_response(self):
        envelope = {"choices": [{"finish_reason": "stop", "message": {"content": json.dumps(proposal())}}]}
        received = []
        mode = ["normal"]

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                received.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
                if mode[0] == "redirect":
                    self.send_response(302)
                    self.send_header("Location", "/redirect-target")
                    self.end_headers()
                    return
                raw = b"x" * 262145 if mode[0] == "oversize" else json.dumps(envelope).encode()
                self.send_response(200)
                self.send_header("Content-Length", str(len(raw)))
                self.end_headers()
                self.wfile.write(raw)

            def log_message(self, *args):
                pass

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        command = [sys.executable, str(Path(__file__).with_name("local_drafter.py")), "--endpoint", f"http://127.0.0.1:{server.server_port}", "--model", "fixture"]
        request = json.dumps({"api_version": "oh.war/draft-request/v1", "user_request": "add a changelog"})
        try:
            good = subprocess.run(command, input=request, text=True, capture_output=True, timeout=10)
            self.assertEqual(good.returncode, 0, good.stderr)
            self.assertEqual(json.loads(good.stdout), proposal())
            self.assertEqual(received[0]["response_format"]["schema"], proposal_schema())
            for response_mode, diagnostic in (("redirect", "redirect refused"), ("oversize", "response exceeds limit")):
                mode[0] = response_mode
                bad = subprocess.run(command, input=request, text=True, capture_output=True, timeout=10)
                self.assertEqual(bad.returncode, 1, bad.stderr)
                self.assertEqual(bad.stdout, "")
                self.assertIn(diagnostic, bad.stderr)
            mode[0] = "normal"
            calls_before = len(received)
            for invalid, diagnostic in ((" " * 65537, "request exceeds 64 KiB"),
                                        ('{"api_version":1,"api_version":2}', "duplicate JSON key")):
                bad = subprocess.run(command, input=invalid, text=True, capture_output=True, timeout=10)
                self.assertEqual(bad.returncode, 1)
                self.assertEqual(bad.stdout, "")
                self.assertIn(diagnostic, bad.stderr)
            self.assertEqual(len(received), calls_before, "invalid input must not contact backend")
            original = copy.deepcopy(envelope)
            envelope["choices"][0]["finish_reason"] = "length"
            bad = subprocess.run(command, input=request, text=True, capture_output=True, timeout=10)
            self.assertEqual(bad.returncode, 1)
            self.assertEqual(bad.stdout, "")
            self.assertIn("complete response", bad.stderr)
            envelope.clear()
            envelope.update(original)
            candidate = proposal()
            candidate["operations"][0]["path"] = "../escape"
            envelope["choices"][0]["message"]["content"] = json.dumps(candidate)
            bad = subprocess.run(command, input=request, text=True, capture_output=True, timeout=10)
            self.assertEqual(bad.returncode, 1)
            self.assertEqual(bad.stdout, "")
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=5)


if __name__ == "__main__":
    unittest.main()
