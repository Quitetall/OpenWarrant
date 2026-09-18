# SPDX-License-Identifier: Apache-2.0
"""Actual bounded process observations, with no model or provider calls."""
import sys
import tempfile
import unittest

from availability import Availability
from execution import ExecutionError


class AvailabilityTests(unittest.TestCase):
    def probe(self, body):
        with tempfile.TemporaryDirectory() as cwd:
            config = {'schema': 'oh.war/availability-config/v1',
                      'argv': [sys.executable, '-c', 'import json,sys,time\nr=json.load(sys.stdin)\n' + body],
                      'timeout_seconds': 1}
            return Availability(config, cwd).observe({'warrant_id': 'fixture'}, 'a' * 64)

    def test_available_unavailable_and_unknown_remain_distinct(self):
        for state in ('available', 'unavailable', 'unknown'):
            with self.subTest(state=state):
                row = self.probe("print(json.dumps({'schema':'oh.war/availability-result/v1', 'nonce':r['nonce'], 'state':" + repr(state) + "}))")
                self.assertEqual(row['state'], state)
                self.assertFalse(row['dispatch_permitted'])
                self.assertFalse(row['qualified'])

    def test_bad_observations_never_mean_available(self):
        for body in ("print('not json')", "print('x' * 5000)", "sys.exit(1)",
                     "print(json.dumps({'schema':'oh.war/availability-result/v1','nonce':'stale','state':'available'}))",
                     "print(json.dumps({'schema':'oh.war/availability-result/v1','nonce':r['nonce'],'state':'available','argv':['sh']}))",
                     "print('{\"state\":\"unknown\",\"state\":\"available\"}')",
                     "print('[' * 1200 + ']' * 1200)", "time.sleep(10)"):
            with self.subTest(body=body):
                self.assertEqual(self.probe(body)['state'], 'unknown')

    def test_configuration_refuses_extra_command_fields(self):
        with self.assertRaises(ExecutionError):
            Availability({'schema':'oh.war/availability-config/v1', 'argv':['true'],
                          'timeout_seconds':1, 'shell':True}, '.')
