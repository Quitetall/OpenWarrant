# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import json
import threading
import unittest
import urllib.error
import urllib.request

from server import Server, publish
from test_verifier_snapshot import SnapshotFixture
from verifier_service import Verification


class VerifierServiceTests(SnapshotFixture, unittest.TestCase):
    def setUp(self):
        super().setUp()
        self.source.update(source="source", source_sha256=hashlib.sha256(b"source").hexdigest())
        self.policy["source_sha256"] = self.source["source_sha256"]
        self.row.update(source_sha256=self.source["source_sha256"], policy=copy.deepcopy(self.policy))
        self.executor.publish = publish
        self.executor.lock = threading.RLock()
        self.service = Verification(self.executor, self.config_path, self.issuer_path)
        self.server = Server(("127.0.0.1", 0), self.executor.store, "fixture-token")
        self.server.verification = self.service
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self.close)
        self.fields = {"attempt_id": self.id, "verification_id": "00000000-0000-4000-8000-000000000003"}

    def close(self):
        self.server.shutdown();self.server.server_close();self.thread.join(timeout=3)

    def call(self, path, data=None, token="fixture-token"):
        headers = {"Authorization": "Bearer " + token}
        if data is not None: headers["Content-Type"] = "application/json"
        req = urllib.request.Request(f"http://127.0.0.1:{self.server.server_port}" + path,
            data=json.dumps(data).encode() if data is not None else None, headers=headers)
        try: response = urllib.request.urlopen(req, timeout=5)
        except urllib.error.HTTPError as error: response = error
        with response: return response.status, json.loads(response.read())

    def test_http_preparation_is_idempotent_and_restart_preserves_request(self):
        code, prepared = self.call('/api/verification', self.fields)
        self.assertEqual(code, 200, prepared)
        self.assertEqual(prepared['state'], 'prepared')
        self.assertFalse(prepared['dispatch_permitted']);self.assertFalse(prepared['qualified'])
        self.assertEqual(self.call('/api/verification', self.fields), (200, prepared))
        self.server.verification = Verification(self.executor, self.config_path, self.issuer_path)
        code, listed = self.call('/api/verification')
        self.assertEqual(code, 200);self.assertEqual(listed['jobs'], [prepared])
        self.assertEqual(self.call('/api/verification/' + self.fields['verification_id']), (200, prepared))

    def test_http_refuses_unlocked_authority_injection_and_changed_identity(self):
        self.assertEqual(self.call('/api/verification', self.fields, token='wrong')[0], 401)
        self.assertEqual(self.call('/api/verification', {**self.fields, 'qualified': True})[0], 409)
        self.assertEqual(self.call('/api/verification')[1]['jobs'], [])
        self.assertEqual(self.call('/api/verification', self.fields)[0], 200)
        self.row['result_revision'] = 'c' * 40
        self.assertEqual(self.call('/api/verification', self.fields)[0], 409)
        self.assertEqual(self.service.get(self.fields['verification_id'])['request']['candidate_revision'], 'b' * 40)

    def test_disabled_service_and_unknown_writer_refuse_without_preparation(self):
        self.row['execution_state'] = 'unknown'
        self.assertEqual(self.call('/api/verification', self.fields)[0], 409)
        self.assertEqual(self.service.listing()['jobs'], [])
        self.server.verification = None
        self.assertEqual(self.call('/api/verification')[0], 409)
