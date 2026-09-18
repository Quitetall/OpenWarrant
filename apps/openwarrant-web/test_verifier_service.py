# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import json
import threading
import base64
import subprocess
import sys
import time
from pathlib import Path
import unittest
import urllib.error
import urllib.request

from server import Server, publish
from test_verifier_snapshot import SnapshotFixture
from verifier_service import Verification
from verifier_attestation import NAMESPACE
from verifier_policy import PROTECTIONS


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

    def dispatch_fixture(self):
        root = self.executor.root
        candidate = Path(self.row['worktree']);candidate.mkdir()
        def git(*args):
            return subprocess.check_output(['git', *args], cwd=candidate, stderr=subprocess.PIPE, text=True).strip()
        git('init', '-q');git('config', 'user.name', 'Fixture');git('config', 'user.email', 'fixture@example.invalid')
        (candidate / 'code.txt').write_text('candidate');git('add', '.');git('commit', '-qm', 'candidate')
        self.row['result_revision'] = git('rev-parse', 'HEAD')
        self.policy['checks'] = [[sys.executable, '-c', "assert open('code.txt').read()=='candidate'"]]
        self.row['policy'] = copy.deepcopy(self.policy)
        self.row['checks'] = [{'argv': self.policy['checks'][0], 'exit_code': 0}]
        program = root / 'reviewer.py'
        program.write_text("""import json,sys,hashlib
r=json.load(sys.stdin)
h=hashlib.sha256(json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
print(json.dumps({'schema':'oh.war/verification-result/v1','verification_id':r['verification_id'],'request_sha256':h,'verdict':'pass','summary':'Fixture passed','findings':[]}))
""")
        self.config['verifier']['argv'] = [sys.executable, str(program)]
        self.config_path.write_text(json.dumps(self.config))
        key = root / 'machine-key'
        subprocess.run(['ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-f', str(key)], check=True)
        public = ' '.join(key.with_suffix('.pub').read_text().split()[:2])
        self.issuer_path.write_text(json.dumps({'schema': 'oh.war/verifier-issuer/v1',
                                               'public_key': public, 'principal': 'fixture'}))
        code, prepared = self.call('/api/verification', self.fields)
        self.assertEqual(code, 200, prepared)
        now = int(time.time())
        payload = json.dumps({'schema': 'oh.war/harness-protection/v1', 'basis_sha256': prepared['basis_sha256'],
            'nonce': self.fields['verification_id'], 'issued_at_unix': now, 'expires_at_unix': now + 120,
            'evidence_ref': 'fixture://synthetic-protections', 'protections': {p: 'pass' for p in PROTECTIONS}}).encode()
        signature = subprocess.run(['ssh-keygen', '-Y', 'sign', '-f', str(key), '-n', NAMESPACE], input=payload,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout
        return {'payload_base64': base64.b64encode(payload).decode(),
                'signature_base64': base64.b64encode(signature).decode()}

    def test_http_signed_dispatch_finishes_and_replay_cannot_launch_again(self):
        fields = self.dispatch_fixture()
        route = '/api/verification/' + self.fields['verification_id']
        code, started = self.call(route + '/start', fields)
        self.assertEqual(code, 202, started)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            code, final = self.call(route)
            if final['state'] == 'finished': break
            time.sleep(.02)
        self.assertEqual(final['state'], 'finished', final)
        self.assertEqual(final['record']['observation']['verdict'], 'pass', final)
        self.assertFalse(final['qualified'])
        (self.executor.root / 'reviewer.py').write_text("raise RuntimeError('must not launch')")
        self.assertEqual(self.call(route + '/start', fields), (202, final))
        self.server.verification = Verification(self.executor, self.config_path, self.issuer_path)
        self.assertEqual(self.call(route + '/start', fields), (202, final))
        receipts = list(self.service.jobs.root.glob('protection-*.json'))
        self.assertEqual(len(receipts), 1)
        original = receipts[0].read_bytes()
        receipts[0].unlink()
        code, missing = self.call(route)
        self.assertEqual(code, 200)
        self.assertEqual(missing['evidence_state'], 'unavailable')
        self.assertEqual(missing['effective_verdict'], 'unknown')
        self.assertEqual(missing['record'], final['record'])
        receipts[0].write_bytes(original)
        self.assertEqual(self.call(route), (200, final))
        receipts[0].write_text('{}')
        self.assertEqual(self.call(route)[1]['effective_verdict'], 'unknown')

    def test_http_forged_receipt_refuses_before_consumption(self):
        fields = self.dispatch_fixture()
        fields['payload_base64'] = base64.b64encode(base64.b64decode(fields['payload_base64']).replace(b'pass', b'fail')).decode()
        route = '/api/verification/' + self.fields['verification_id']
        self.assertEqual(self.call(route + '/start', fields)[0], 409)
        self.assertEqual(self.call(route)[1]['state'], 'prepared')
        self.assertFalse((self.service.jobs.root / ('workspace-' + self.fields['verification_id'])).exists())

    def test_interrupted_consumed_job_blocks_new_dispatch(self):
        fields = self.dispatch_fixture()
        self.service.jobs.begin(self.fields['verification_id'], 'e' * 64)
        prior = self.service.get(self.fields['verification_id'])
        self.assertEqual(prior['state'], 'unknown')
        next_fields = {**self.fields, 'verification_id': '00000000-0000-4000-8000-000000000004'}
        self.assertEqual(self.call('/api/verification', next_fields)[0], 200)
        route = '/api/verification/' + next_fields['verification_id']
        self.assertEqual(self.call(route + '/start', fields)[0], 409)
        self.assertEqual(self.call(route)[1]['state'], 'prepared')
