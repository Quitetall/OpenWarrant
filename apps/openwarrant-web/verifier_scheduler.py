# SPDX-License-Identifier: Apache-2.0
"""Opt-in loop scheduling; durable intent, existing dispatch gates, explicit pause."""
import json
import threading

from hotline import digest
from verification import identity, require
from verifier_loop import Loop


class Scheduler:
    def __init__(self, service, receipts, interval=2):
        require(type(interval) in (int, float) and .01 <= interval <= 60, 'Invalid loop poll interval')
        self.service, self.receipts, self.interval = service, receipts, interval
        self.executor = service.executor
        self.root = self.executor.root / 'verification-loops'
        require(not self.root.is_symlink(), 'Loop directory cannot be a symlink')
        self.root.mkdir(mode=0o700, exist_ok=True)
        require(self.root.is_dir() and self.root.stat().st_mode & 0o077 == 0, 'Private loop directory required')
        self.observations = {}
        self.stop = threading.Event()
        self.thread = None
        self.loop = Loop(service, receipts)

    def records(self):
        result = {}
        for path in sorted(self.root.glob('*.json')):
            parts = path.name.split('.')
            require(len(parts) == 3 and identity(parts[0]) and parts[1].isdigit()
                    and len(parts[1]) == 6, 'Invalid loop record filename')
            id, sequence = parts[0], int(parts[1])
            envelope = self.executor.decode(self.executor.read_file(path, 8192))
            require(isinstance(envelope, dict) and set(envelope) == {'record', 'sha256'}, 'Invalid loop envelope')
            row = envelope['record']; previous = result.get(id)
            require(isinstance(row, dict) and set(row) == {'schema', 'attempt_id', 'sequence', 'previous_sha256', 'enabled'}
                    and row['schema'] == 'oh.war/verifier-loop-intent/v1' and row['attempt_id'] == id
                    and type(row['sequence']) is int and row['sequence'] == sequence
                    and type(row['enabled']) is bool and digest(row) == envelope['sha256']
                    and sequence == (previous['sequence'] + 1 if previous else 1)
                    and row['previous_sha256'] == (digest(previous) if previous else None), 'Loop intent history mismatch')
            result[id] = row
            require(len(result) <= 64 and sequence <= 1024, 'Loop history limit exceeded')
        return result

    def configure(self, fields):
        require(isinstance(fields, dict) and set(fields) == {'attempt_id', 'enabled'}
                and identity(fields['attempt_id']) and type(fields['enabled']) is bool, 'Exact loop attempt and enabled flag required')
        with self.executor.lock:
            require(not self.stop.is_set(), 'Loop scheduler closed')
            records = self.records(); id = fields['attempt_id']; previous = records.get(id)
            attempt = self.executor.get(id)
            if fields['enabled']:
                require(attempt['work_state'] == 'completed' and attempt['execution_state'] == 'stopped',
                        'Completed stopped root attempt required')
                for other, row in records.items():
                    require(other == id or not row['enabled'] or self.executor.get(other)['warrant_id'] != attempt['warrant_id'],
                            'Another loop already enabled for this Warrant')
            if previous and previous['enabled'] == fields['enabled']:
                return self.view(id, previous)
            require(previous is not None or len(records) < 64, 'Loop inventory limit exceeded')
            row = {'schema': 'oh.war/verifier-loop-intent/v1', 'attempt_id': id,
                   'sequence': previous['sequence'] + 1 if previous else 1,
                   'previous_sha256': digest(previous) if previous else None, 'enabled': fields['enabled']}
            require(row['sequence'] <= 1024, 'Loop history limit exceeded')
            self.executor.publish(self.root / (id + '.' + str(row['sequence']).zfill(6) + '.json'),
                                  json.dumps({'record': row, 'sha256': digest(row)}).encode())
            self.observations.pop(id, None)
            return self.view(id, row)

    def view(self, id, row):
        return {'attempt_id': id, 'enabled': row['enabled'], 'intent_sequence': row['sequence'],
                'observation': self.observations.get(id), 'qualified': False,
                'state': 'scheduled' if row['enabled'] else 'paused'}

    def listing(self):
        with self.executor.lock:
            return {'schema': 'oh.war/verifier-loops/v1', 'qualified': False,
                    'loops': [self.view(id, row) for id, row in self.records().items()]}

    def poll(self):
        with self.executor.lock:
            if self.stop.is_set(): return
            for id, row in self.records().items():
                if not row['enabled'] or self.stop.is_set(): continue
                try:
                    self.observations[id] = self.loop.tick(id)
                except Exception as error:
                    self.observations[id] = {'state': 'needs_attention', 'reason': str(error)[:1000], 'qualified': False}

    def start(self):
        require(self.thread is None and not self.stop.is_set(), 'Scheduler already started or closed')
        def worker():
            while not self.stop.wait(self.interval):
                try: self.poll()
                except Exception:
                    # Corrupt intent history cannot authorize any more dispatch.
                    self.stop.set()
        self.thread = threading.Thread(target=worker, daemon=True)
        self.thread.start()

    def close(self):
        self.stop.set()
        if self.thread:
            self.thread.join(timeout=30)
            require(not self.thread.is_alive(), 'Loop scheduler has not stopped')
