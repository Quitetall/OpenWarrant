# SPDX-License-Identifier: Apache-2.0
"""Durable queue state and crash boundaries; synthetic executor seam."""
import json
import tempfile
import threading
import unittest
from pathlib import Path

from dispatch_queue import DispatchQueue
from execution import ExecutionError
from server import read_file, publish, decode


class QueueTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.subject = {'warrant_id': 'fixture', 'source_sha256': 'a' * 64}
        test = self
        class Executor:
            root = Path(test.tmp.name)
            lock = threading.RLock()
            config = {'warrants': {'fixture': {'source_sha256': 'a' * 64}}}
            read_file, publish, decode = staticmethod(read_file), staticmethod(publish), staticmethod(decode)
            store = type('Store', (), {'get': lambda _, id: {'source_sha256': 'a' * 64}})()
            attempts = {}; launches = 0; ready = True; crash = None
            def admission(self, subject):
                return {'state': 'ready' if self.ready else 'blocked', 'reason': 'fixture'}
            def records(self): return self.attempts
            def start(self, subject, queue_context=None):
                self.launches += 1
                if self.crash == 'before': raise RuntimeError('before')
                row = {'attempt_id': '00000000-0000-0000-0000-000000000001',
                       'queue_dispatch': queue_context}
                self.attempts[row['attempt_id']] = row
                if self.crash == 'after': raise RuntimeError('after')
                return row
        class Probe:
            state = 'unavailable'
            calls = 0
            def observe(self, *args):
                self.calls += 1
                return {'state': self.state, 'reason': 'fixture'}
        self.executor, self.probe = Executor(), Probe()
        self.queue = DispatchQueue(self.executor, self.probe)

    def row(self): return self.queue.listing()['requests'][0]

    def test_wait_then_dispatch_once_and_restart(self):
        first = self.queue.enqueue(self.subject)
        self.assertEqual(self.queue.enqueue(self.subject), first)
        self.queue.poll()
        self.assertEqual(self.row()['state'], 'waiting_for_agent')
        self.assertEqual(self.executor.launches, 0)
        self.probe.state = 'available'; self.queue.poll(); self.queue.poll()
        self.assertEqual(self.row()['state'], 'dispatched')
        self.queue = DispatchQueue(self.executor, self.probe); self.queue.poll()
        self.assertEqual(self.executor.launches, 1)

    def test_consumed_crash_never_replays_with_or_without_attempt(self):
        for point in ('before', 'after'):
            with self.subTest(point=point):
                # Independent store per crash scenario.
                self.setUp()
                self.queue.enqueue(self.subject); self.probe.state = 'available'
                self.executor.crash = point; self.queue.poll()
                self.assertEqual(self.row()['durable_state'], 'consumed')
                self.queue = DispatchQueue(self.executor, self.probe); self.queue.poll()
                self.assertEqual(self.row()['state'], 'unknown' if point == 'before' else 'dispatched')
                self.assertEqual(self.executor.launches, 1)

    def test_one_probe_per_poll_rotates_waiting_work(self):
        self.queue.enqueue({**self.subject, 'stage': 'one'})
        self.queue.enqueue({**self.subject, 'stage': 'two'})
        seen = []
        def observe(subject, config):
            seen.append(subject['stage'])
            return {'state': 'unavailable', 'reason': 'fixture'}
        self.probe.observe = observe
        self.queue.poll(); self.assertEqual(len(seen), 1)
        self.queue.poll(); self.assertEqual(set(seen), {'one', 'two'})
        self.assertEqual(self.executor.launches, 0)

    def test_cancel_survives_restart(self):
        row = self.queue.enqueue(self.subject)
        self.queue.cancel(row['queue_id']); self.probe.state = 'available'
        self.queue = DispatchQueue(self.executor, self.probe); self.queue.poll()
        self.assertEqual(self.row()['state'], 'cancelled')
        self.assertEqual(self.executor.launches, 0)

    def test_current_admission_and_configuration_block(self):
        self.queue.enqueue(self.subject); self.probe.state = 'available'
        self.executor.ready = False; self.queue.poll()
        self.assertEqual(self.probe.calls, 0)
        self.executor.ready = True; self.executor.config['changed'] = True
        self.queue.poll()
        self.assertEqual(self.executor.launches, 0)
        self.assertIn('configuration changed', self.row()['observation']['reason'])

    def test_unknown_and_corrupt_history_cannot_dispatch(self):
        self.queue.enqueue(self.subject); self.probe.state = 'unknown'; self.queue.poll()
        self.assertEqual(self.executor.launches, 0)
        path = next(self.queue.root.glob('*.json'))
        envelope = json.loads(path.read_text()); envelope['record']['state'] = 'dispatched'
        path.write_text(json.dumps(envelope)); self.probe.state = 'available'
        with self.assertRaises(ExecutionError): self.queue.poll()
        self.assertEqual(self.executor.launches, 0)


class QueueExecutionTests(unittest.TestCase):
    def test_real_executor_retains_queue_binding_and_runs_required_checks(self):
        import os
        import time
        import test_execution
        from execution import Executor
        from server import Store, SDK
        fixture = test_execution.ExecutionTests(); fixture.setUp()
        self.addCleanup(fixture.tearDown)
        draft = fixture.eligible(); fixture.stop()
        store = Store(fixture.root / 'state', SDK(Path(test_execution.WAR), fixture.repo))
        self.addCleanup(lambda: os.close(store.lock_fd))
        executor = Executor(store, fixture.config_path, read_file, publish, decode)
        probe = type('Probe', (), {'observe': lambda *args: {'state': 'available', 'reason': 'synthetic'}})()
        queue = DispatchQueue(executor, probe)
        queued = queue.enqueue({'warrant_id': draft['id'], 'source_sha256': draft['source_sha256']})
        queue.poll()
        row = queue.listing()['requests'][0]
        self.assertEqual(row['state'], 'dispatched')
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            attempt = executor.get(row['attempt_id'])
            if attempt['execution_state'] != 'running': break
            time.sleep(.02)
        self.assertEqual(attempt['work_state'], 'completed', attempt)
        self.assertEqual(attempt['queue_dispatch']['queue_id'], queued['queue_id'])
        self.assertTrue(attempt['checks'])
        queue = DispatchQueue(executor, probe); queue.poll()
        self.assertEqual(len(executor.records()), 1)

    def test_real_admission_gates_remain_binding_for_queued_work(self):
        import os
        import test_execution
        from execution import Executor
        from server import Store, SDK
        cases = {'verified': 'Verified-start', 'cost': 'Unknown cost',
                 'dependency': 'Required dependency', 'writer': 'Existing writer',
                 'source': 'Changed subject'}
        for case, reason in cases.items():
            with self.subTest(case=case):
                fixture = test_execution.ExecutionTests(); fixture.setUp()
                try:
                    draft = fixture.eligible(); fixture.stop()
                    policy = fixture.config['warrants'][draft['id']]
                    if case == 'verified': policy['verified_start'] = True
                    if case == 'cost': fixture.config['cost_mode'] = 'unknown'
                    if case == 'dependency': policy['dependencies'] = [draft['id']]
                    fixture.config_path.write_text(json.dumps(fixture.config))
                    store = Store(fixture.root / 'state', SDK(Path(test_execution.WAR), fixture.repo))
                    try:
                        executor = Executor(store, fixture.config_path, read_file, publish, decode)
                        def probe(*args): self.fail('Blocked subject reached availability probe')
                        queue = DispatchQueue(executor, type('Probe', (), {'observe': probe})())
                        subject = {'warrant_id': draft['id'], 'source_sha256': draft['source_sha256']}
                        queue.enqueue(subject)
                        if case == 'writer':
                            executor.save({'attempt_id':'00000000-0000-0000-0000-000000000002',
                                'sequence':1,'warrant_id':draft['id'],'execution_state':'running',
                                'work_state':'in-progress'})
                        if case == 'source':
                            fields = {key:draft[key] for key in ('id','title','outcome','scope','context')}
                            fields['outcome'] = 'Changed outcome'
                            store.create(fields, draft['source_sha256'])
                        before = len(executor.records()); queue.poll()
                        row = queue.listing()['requests'][0]
                        self.assertEqual(row['state'], 'blocked', row)
                        self.assertIn(reason, row['observation']['reason'])
                        self.assertEqual(len(executor.records()), before)
                        self.assertEqual(row['durable_state'], 'queued')
                    finally: os.close(store.lock_fd)
                finally: fixture.tearDown()

    def test_queued_stages_wait_for_checked_predecessor_in_one_worktree(self):
        import os
        import time
        import test_execution
        from execution import Executor
        from server import Store, SDK
        fixture = test_execution.ExecutionTests(); fixture.setUp()
        self.addCleanup(fixture.tearDown)
        subject = fixture.staged(); fixture.stop()
        store = Store(fixture.root / 'state', SDK(Path(test_execution.WAR), fixture.repo))
        self.addCleanup(lambda: os.close(store.lock_fd))
        executor = Executor(store, fixture.config_path, read_file, publish, decode)
        probe = type('Probe', (), {'observe':lambda *args:{'state':'available','reason':'synthetic'}})()
        queue = DispatchQueue(executor, probe)
        ui = queue.enqueue({**subject, 'stage':'ui'})
        queue.poll()
        self.assertEqual(queue.listing()['requests'][0]['state'], 'blocked')
        self.assertEqual(executor.records(), {})
        api = queue.enqueue({**subject, 'stage':'api'})
        deadline = time.monotonic()+8
        while time.monotonic()<deadline:
            queue.poll()
            attempts = list(executor.records().values())
            if len(attempts)==2 and all(executor.view(r)['execution_state']=='stopped' for r in attempts): break
            time.sleep(.02)
        self.assertEqual(len(attempts),2)
        ordered = sorted(attempts,key=lambda r:r['created_at_unix'])
        self.assertEqual([r['stage'] for r in ordered],['api','ui'])
        self.assertEqual([r['work_state'] for r in ordered],['in-progress','completed'])
        self.assertEqual(ordered[0]['worktree'],ordered[1]['worktree'])
        self.assertEqual([r['queue_dispatch']['queue_id'] for r in ordered],[api['queue_id'],ui['queue_id']])
