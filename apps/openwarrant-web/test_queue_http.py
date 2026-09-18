# SPDX-License-Identifier: Apache-2.0
"""Actual API, background availability, Git and process dispatch."""
import json
import sys
import time
import unittest
import test_execution


class QueueHTTPTests(unittest.TestCase):
    def setUp(self):
        self.fixture = t = test_execution.ExecutionTests(); t.setUp()
        self.addCleanup(t.tearDown)
        self.draft = t.eligible(); t.stop()
        self.marker = t.root / 'available'
        probe = t.root / 'probe.py'
        probe.write_text("import json,sys,pathlib\nr=json.load(sys.stdin)\n"
            + "print(json.dumps({'schema':'oh.war/availability-result/v1','nonce':r['nonce'],"
            + "'state':'available' if pathlib.Path(" + repr(str(self.marker)) + ").exists() else 'unavailable'}))\n")
        config = t.root / 'availability.json'
        config.write_text(json.dumps({'schema':'oh.war/availability-config/v1',
                                     'argv':[sys.executable,str(probe)],'timeout_seconds':1}))
        t.report_args = ['--availability-config',str(config)]; t.start()
        self.subject = {'warrant_id':self.draft['id'],'source_sha256':self.draft['source_sha256']}

    def wait_state(self, expected):
        t = self.fixture; deadline = time.monotonic()+8
        while time.monotonic()<deadline:
            code, data = t.call('/api/queue'); self.assertEqual(code,200,data)
            if data['requests'] and data['requests'][0]['state']==expected:return data['requests'][0]
            time.sleep(.05)
        self.fail(str(data))

    def test_auth_wait_automatic_dispatch_and_restart(self):
        t=self.fixture
        self.assertEqual(t.call('/api/queue','POST',self.subject,{'Authorization':'Bearer wrong'})[0],401)
        self.assertEqual(t.call('/api/queue','POST',{**self.subject,'argv':['sh']})[0],400)
        code,row=t.call('/api/queue','POST',self.subject);self.assertEqual(code,202,row)
        self.wait_state('waiting_for_agent')
        self.assertEqual(t.call('/api/runs')[1]['runs'],[])
        self.marker.touch();row=self.wait_state('dispatched')
        attempt=t.wait_run(row['attempt_id']);self.assertEqual(attempt['work_state'],'completed',attempt)
        t.stop();t.start();self.wait_state('dispatched')
        self.assertEqual(len(t.call('/api/runs')[1]['runs']),1)

    def test_cancelled_work_never_launches_after_restart(self):
        t=self.fixture
        code,row=t.call('/api/queue','POST',self.subject);self.assertEqual(code,202,row)
        self.assertEqual(t.call('/api/queue/cancel','POST',{'queue_id':row['queue_id'],'all':True})[0],400)
        self.assertEqual(t.call('/api/queue/cancel','POST',{'queue_id':row['queue_id']})[0],200)
        t.stop();self.marker.touch();t.start();time.sleep(2.2)
        self.wait_state('cancelled');self.assertEqual(t.call('/api/runs')[1]['runs'],[])
