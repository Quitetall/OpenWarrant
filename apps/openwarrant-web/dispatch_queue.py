# SPDX-License-Identifier: Apache-2.0
"""Durable one-shot dispatch requests; uncertain consumption never replays."""
import json
import re
import threading
import uuid

from execution import require
from hotline import digest


class DispatchQueue:
    def __init__(self, executor, availability):
        self.executor, self.availability = executor, availability
        self.root = executor.root / 'queue'
        require(not self.root.is_symlink(), 'Queue directory cannot be a symlink')
        self.root.mkdir(mode=0o700, exist_ok=True)
        require(self.root.stat().st_mode & 0o077 == 0, 'Private queue directory required')
        self.observations = {}
        self.cursor = None
        self.stop = threading.Event()
        self.thread = None
        self.scheduler_error = None

    def records(self):
        paths = list(self.root.iterdir())
        require(len(paths) <= 768, 'Queue history limit exceeded')
        rows = {}
        for path in sorted(paths):
            match = re.fullmatch(r'([0-9a-f-]{36})\.([123])\.json', path.name)
            require(match is not None, 'Unknown queue record')
            id, sequence = match[1], int(match[2])
            require(str(uuid.UUID(id)) == id, 'Invalid queue identity')
            envelope = self.executor.decode(self.executor.read_file(path, 16384))
            require(isinstance(envelope, dict) and set(envelope) == {'record', 'sha256'}, 'Invalid queue envelope')
            row = envelope['record']; previous = rows.get(id)
            require(isinstance(row, dict) and set(row) == {'schema', 'queue_id', 'sequence',
                    'previous_sha256', 'state', 'subject', 'execution_config_sha256', 'attempt_id'}
                    and row['schema'] == 'oh.war/dispatch-queue-record/v1'
                    and row['queue_id'] == id and type(row['sequence']) is int
                    and row['sequence'] == sequence and envelope['sha256'] == digest(row)
                    and sequence == (previous['sequence'] + 1 if previous else 1)
                    and row['previous_sha256'] == (digest(previous) if previous else None),
                    'Queue history mismatch')
            require(isinstance(row['subject'], dict)
                    and set(row['subject']) in ({'warrant_id', 'source_sha256'}, {'warrant_id', 'source_sha256', 'stage'})
                    and all(isinstance(v, str) and v for v in row['subject'].values())
                    and re.fullmatch('[0-9a-f]{64}', row['subject']['source_sha256'])
                    and isinstance(row['execution_config_sha256'], str)
                    and re.fullmatch('[0-9a-f]{64}', row['execution_config_sha256']), 'Invalid queue subject')
            require((sequence == 1 and row['state'] == 'queued' and row['attempt_id'] is None)
                    or (sequence == 2 and previous['state'] == 'queued'
                        and row['state'] in ('cancelled', 'consumed') and row['attempt_id'] is None)
                    or (sequence == 3 and previous['state'] == 'consumed'
                        and row['state'] == 'dispatched' and isinstance(row['attempt_id'], str)
                        and re.fullmatch('[0-9a-f-]{36}', row['attempt_id'])), 'Invalid queue transition')
            if previous:
                require(row['subject'] == previous['subject']
                        and row['execution_config_sha256'] == previous['execution_config_sha256'],
                        'Queue subject changed')
            rows[id] = row
        require(len(rows) <= 256, 'Queue inventory limit exceeded')
        return rows

    def save(self, row):
        self.executor.publish(self.root / f"{row['queue_id']}.{row['sequence']}.json",
                              json.dumps({'record': row, 'sha256': digest(row)}).encode())

    def transition(self, row, state, attempt=None):
        next_row = {**row, 'sequence': row['sequence'] + 1, 'previous_sha256': digest(row),
                    'state': state, 'attempt_id': attempt}
        self.save(next_row)
        return next_row

    def enqueue(self, subject):
        with self.executor.lock:
            require(not self.stop.is_set(), 'Queue scheduler closed')
            # This validates field shape; blocked prerequisites can remain queued.
            self.executor.admission(subject)
            policy = self.executor.config['warrants'].get(subject['warrant_id'])
            require(policy is not None and subject['source_sha256'] == policy['source_sha256']
                    == self.executor.store.get(subject['warrant_id'])['source_sha256'], 'Queue requires current configured source')
            config = digest(self.executor.config)
            rows = self.records()
            for row in rows.values():
                if row['subject'] == subject and row['execution_config_sha256'] == config:
                    if row['state'] != 'cancelled': return self.view(row)
                require(row['state'] in ('cancelled', 'dispatched') or
                        (row['subject']['warrant_id'], row['subject'].get('stage')) !=
                        (subject['warrant_id'], subject.get('stage')), 'Pending request for this work already exists')
            require(len(rows) < 256, 'Queue inventory limit exceeded')
            row = {'schema': 'oh.war/dispatch-queue-record/v1', 'queue_id': str(uuid.uuid4()),
                   'sequence': 1, 'previous_sha256': None, 'state': 'queued',
                   'subject': dict(subject), 'execution_config_sha256': config, 'attempt_id': None}
            self.save(row)
            return self.view(row)

    def cancel(self, id):
        with self.executor.lock:
            rows = self.records(); require(id in rows, 'Unknown queue request', 404)
            row = rows[id]
            require(row['state'] in ('queued', 'cancelled'), 'Dispatch already consumed; cancellation cannot stop work')
            if row['state'] == 'queued': row = self.transition(row, 'cancelled')
            self.observations.pop(id, None)
            return self.view(row)

    def view(self, row):
        observation = self.observations.get(row['queue_id'])
        state = row['state']
        if state == 'consumed': state = 'unknown'
        if state == 'queued' and observation: state = observation['state']
        attempt_path = '/api/runs/' + row['attempt_id'] if row['attempt_id'] else None
        return {**row, 'state': state, 'durable_state': row['state'],
                'attempt_path': attempt_path,
                'report_path': attempt_path + '/report' if attempt_path else None,
                'observation': observation, 'qualified': False}

    def listing(self):
        with self.executor.lock:
            return {'schema': 'oh.war/dispatch-queue/v1', 'qualified': False,
                    'scheduler_error': self.scheduler_error,
                    'requests': [self.view(row) for row in self.records().values()]}

    def poll(self):
        with self.executor.lock:
            rows = list(self.records().items())
            ids = [id for id, _ in rows]
            if self.cursor in ids:
                split = ids.index(self.cursor) + 1
                rows = rows[split:] + rows[:split]
            probed = False
            for id, row in rows:
                if self.stop.is_set(): return
                if row['state'] == 'consumed':
                    # Recover only an exact durable association. No second launch.
                    binding = {'queue_id': id, 'consumed_sha256': digest(row)}
                    matches = [r for r in self.executor.records().values() if r.get('queue_dispatch') == binding]
                    if len(matches) == 1:
                        self.transition(row, 'dispatched', matches[0]['attempt_id'])
                    continue
                if row['state'] != 'queued': continue
                if row['execution_config_sha256'] != digest(self.executor.config):
                    self.observations[id] = {'state': 'blocked', 'reason': 'Execution configuration changed; cancel and enqueue again'}
                    continue
                admission = self.executor.admission(row['subject'])
                if admission['state'] != 'ready':
                    self.observations[id] = admission
                    continue
                if probed: continue
                # One bounded probe per poll; rotate so an unavailable agent's
                # request cannot starve other eligible work or hold the lock forever.
                probed = True
                self.cursor = id
                available = self.availability.observe(row['subject'], row['execution_config_sha256'])
                self.observations[id] = {**available, 'state': 'waiting_for_agent' if available['state'] == 'unavailable' else available['state']}
                if self.stop.is_set(): return
                if available['state'] != 'available': continue
                # Save before any dispatch side effect. A thrown exception leaves
                # consumed UNKNOWN, even when it happened before process creation.
                consumed = self.transition(row, 'consumed')
                self.observations.pop(id, None)
                try:
                    attempt = self.executor.start(row['subject'], queue_context={
                        'queue_id': id, 'consumed_sha256': digest(consumed)})
                    self.transition(consumed, 'dispatched', attempt['attempt_id'])
                except Exception:
                    self.observations[id] = {'state': 'unknown', 'reason': 'Dispatch outcome uncertain; inspect retained attempt before recovery'}

    def start(self, interval=2):
        require(type(interval) in (int, float) and .02 <= interval <= 60,
                'Invalid queue polling interval')
        require(self.thread is None and not self.stop.is_set(), 'Queue scheduler already started or closed')
        def worker():
            while not self.stop.wait(interval):
                try:
                    self.poll()
                except Exception:
                    self.scheduler_error = 'Queue state unavailable or invalid; scheduler stopped'
                    self.stop.set()
        self.thread = threading.Thread(target=worker, daemon=True)
        self.thread.start()

    def close(self):
        self.stop.set()
        if self.thread is not None:
            self.thread.join(timeout=10)
