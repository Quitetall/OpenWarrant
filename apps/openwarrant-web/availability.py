# SPDX-License-Identifier: Apache-2.0
"""Owner-configured availability observation; never an execution grant."""
import json
import subprocess
import time
import uuid

from execution import require
from harness import argv, bounded_command


class Availability:
    def __init__(self, config, cwd):
        require(isinstance(config, dict) and set(config) == {'schema', 'argv', 'timeout_seconds'}
                and config['schema'] == 'oh.war/availability-config/v1'
                and argv(config['argv']) and type(config['timeout_seconds']) is int
                and 1 <= config['timeout_seconds'] <= 5,
                'Invalid availability configuration', 400)
        # Copy inputs so later caller mutation cannot change the configured command.
        self.command = list(config['argv'])
        self.timeout = config['timeout_seconds']
        self.cwd = cwd

    def observe(self, subject, configuration_sha256):
        nonce = str(uuid.uuid4())
        request = {'schema': 'oh.war/availability-request/v1', 'nonce': nonce,
                   'subject': subject, 'execution_config_sha256': configuration_sha256}
        state, reason = 'unknown', 'Availability observation unavailable'
        try:
            code, out, _ = bounded_command(self.command, self.cwd,
                json.dumps(request).encode(), time.monotonic() + self.timeout, {})
            if code == 0 and len(out) <= 4096:
                # Reject duplicate fields, including conflicting state claims.
                def unique(pairs):
                    result = {}
                    for key, value in pairs:
                        if key in result: raise ValueError('Duplicate field')
                        result[key] = value
                    return result
                row = json.loads(out, object_pairs_hook=unique)
                if (isinstance(row, dict) and set(row) == {'schema', 'nonce', 'state'}
                        and row['schema'] == 'oh.war/availability-result/v1'
                        and row['nonce'] == nonce
                        and row['state'] in ('available', 'unavailable', 'unknown')):
                    state = row['state']
                    reason = {'available': 'Configured agent reports availability',
                              'unavailable': 'Waiting for configured agent',
                              'unknown': 'Configured agent availability unknown'}[state]
        except (OSError, ValueError, TypeError, TimeoutError, RecursionError, subprocess.SubprocessError):
            pass
        return {'state': state, 'reason': reason, 'dispatch_permitted': False,
                'qualified': False}
